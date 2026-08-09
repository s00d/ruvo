#!/usr/bin/env bash
# Framework comparison stand: byte-identical bodies + oha load + regression gate.
#
# Always builds --release (production profile with workspace LTO).
#
# Stability (why old GET / looked “91–117k”):
#   - First framework in a cold run paid CPU/cache tax → fake gap.
#   - Fix: oha warm-up at full concurrency, rotate who runs first each round,
#     take **median** across ROUNDS (default 3).
#
# Usage:
#   ./bench/stand/run.sh              # verify + deep load, write results
#   ./bench/stand/run.sh --update-baseline
#   DURATION=60s CONCURRENCY=200 ROUNDS=5 ./bench/stand/run.sh
#   PROFILE=quick ./bench/stand/run.sh   # shorter smoke
#
# Requires: oha, curl, python3, cargo
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
RESULTS="$ROOT/results"
PROFILE="${PROFILE:-deep}"
case "$PROFILE" in
  quick)
    DURATION="${DURATION:-8s}"
    CONCURRENCY="${CONCURRENCY:-50}"
    WARMUP="${WARMUP:-3s}"
    ROUNDS="${ROUNDS:-1}"
    ;;
  *)
    DURATION="${DURATION:-15s}"
    CONCURRENCY="${CONCURRENCY:-100}"
    WARMUP="${WARMUP:-5s}"
    ROUNDS="${ROUNDS:-3}"
    ;;
esac
TOKIO_WORKER_THREADS="${TOKIO_WORKER_THREADS:-4}"
UPDATE_BASELINE=0
REGRESSION_RPS_PCT="${REGRESSION_RPS_PCT:-15}"
REGRESSION_P99_PCT="${REGRESSION_P99_PCT:-40}"

for arg in "$@"; do
  case "$arg" in
    --update-baseline) UPDATE_BASELINE=1 ;;
    -h|--help)
      sed -n '2,18p' "$0"
      exit 0
      ;;
  esac
done

if ! command -v oha >/dev/null 2>&1; then
  echo "install oha: cargo install oha" >&2
  exit 1
fi

mkdir -p "$RESULTS"
cd "$ROOT"

echo "==> building stand binaries (release / production)"
TOKIO_WORKER_THREADS="$TOKIO_WORKER_THREADS" cargo build --release -q \
  -p stand_sova -p stand_axum -p stand_actix

PIDS=()
cleanup() {
  for pid in "${PIDS[@]:-}"; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
}
trap cleanup EXIT

start_one() {
  local name="$1" port="$2" bin="$3"
  echo "==> starting $name on :$port"
  PORT="$port" TOKIO_WORKER_THREADS="$TOKIO_WORKER_THREADS" \
    "$ROOT/target/release/$bin" >"$RESULTS/$name.log" 2>&1 &
  PIDS+=($!)
  for _ in $(seq 1 80); do
    if curl -sf "http://127.0.0.1:$port/api/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  echo "failed to start $name (see $RESULTS/$name.log)" >&2
  tail -n 40 "$RESULTS/$name.log" >&2 || true
  exit 1
}

start_one sova 9101 stand_sova
start_one axum 9102 stand_axum
start_one actix 9103 stand_actix

PATHS=(/ /about /blog /blog/hello /contact /api/health)
ECHO_BODY="$(cat "$ROOT/fixtures/echo.json")"
FWS=(sova axum actix)
PORTS=(9101 9102 9103)

echo "==> verifying byte-identical response bodies (GET + POST echo)"
VERIFY_JSON="$RESULTS/verify.json"
python3 - "$VERIFY_JSON" "$ECHO_BODY" <<'PY'
import hashlib, json, sys, urllib.request

out_path = sys.argv[1]
echo_body = sys.argv[2].encode()
ports = {"sova": 9101, "axum": 9102, "actix": 9103}
paths = ["/", "/about", "/blog", "/blog/hello", "/contact", "/api/health"]

def fetch(port, path, data=None):
    url = f"http://127.0.0.1:{port}{path}"
    if data is None:
        req = urllib.request.Request(url)
    else:
        req = urllib.request.Request(
            url,
            data=data,
            method="POST",
            headers={"Content-Type": "application/json"},
        )
    with urllib.request.urlopen(req, timeout=5) as r:
        body = r.read()
        ctype = r.headers.get("Content-Type", "")
        return body, ctype

report = {"ok": True, "paths": {}}
for path in paths:
    bodies = {}
    ctypes = {}
    for name, port in ports.items():
        body, ctype = fetch(port, path)
        bodies[name] = body
        ctypes[name] = ctype
    hashes = {n: hashlib.sha256(b).hexdigest() for n, b in bodies.items()}
    same = len(set(hashes.values())) == 1
    entry = {
        "ok": same,
        "sha256": hashes,
        "bytes": {n: len(b) for n, b in bodies.items()},
        "content_type": ctypes,
    }
    if not same:
        report["ok"] = False
        names = list(bodies)
        a, b = names[0], names[1]
        entry["diff"] = f"{a} vs {b}: {hashes[a]} != {hashes[b]}"
    report["paths"][path] = entry

bodies = {}
ctypes = {}
for name, port in ports.items():
    body, ctype = fetch(port, "/api/echo", data=echo_body)
    bodies[name] = body
    ctypes[name] = ctype
hashes = {n: hashlib.sha256(b).hexdigest() for n, b in bodies.items()}
same = len(set(hashes.values())) == 1 and bodies["sova"] == echo_body
entry = {
    "ok": same,
    "sha256": hashes,
    "bytes": {n: len(b) for n, b in bodies.items()},
    "content_type": ctypes,
    "echo_matches_request": bodies.get("sova") == echo_body,
}
if not same:
    report["ok"] = False
    entry["diff"] = "echo body mismatch across frameworks or vs request"
report["paths"]["POST /api/echo"] = entry

with open(out_path, "w") as f:
    json.dump(report, f, indent=2)
    f.write("\n")

if not report["ok"]:
    print("BODY MISMATCH", json.dumps(report, indent=2))
    sys.exit(1)
print("bodies match for all paths across sova/axum/actix (incl. POST /api/echo)")
PY

# oha at full concurrency — curl warm-up is useless for this load shape.
oha_quiet() {
  local port="$1" path="$2" dur="$3" method="${4:-GET}" body_file="${5:-}"
  local url="http://127.0.0.1:${port}${path}"
  if [[ "$method" == "POST" ]]; then
    oha -z "$dur" -c "$CONCURRENCY" -m POST -T 'application/json' -D "$body_file" --no-tui "$url" >/dev/null 2>&1 || true
  else
    oha -z "$dur" -c "$CONCURRENCY" --no-tui "$url" >/dev/null 2>&1 || true
  fi
}

echo "==> oha warm-up WARMUP=$WARMUP c=$CONCURRENCY (/, /api/health, POST /api/echo × each fw)"
for i in 0 1 2; do
  port="${PORTS[$i]}"
  echo "  warm ${FWS[$i]} :$port" >&2
  oha_quiet "$port" "/" "$WARMUP"
  oha_quiet "$port" "/api/health" "$WARMUP"
  oha_quiet "$port" "/api/echo" "$WARMUP" POST "$ROOT/fixtures/echo.json"
done

oha_one() {
  local name="$1" port="$2" path="$3" round="$4"
  local method="${5:-GET}"
  local body_file="${6:-}"
  local url="http://127.0.0.1:${port}${path}"
  local raw
  if [[ "$method" == "POST" ]]; then
    raw="$(oha -z "$DURATION" -c "$CONCURRENCY" -m POST -T 'application/json' -D "$body_file" --no-tui "$url" 2>&1)" || true
  else
    raw="$(oha -z "$DURATION" -c "$CONCURRENCY" --no-tui "$url" 2>&1)" || true
  fi
  local rps p50 p99 success
  rps="$(echo "$raw" | awk '/Requests\/sec:/ {print $NF; exit}')"
  p50="$(echo "$raw" | awk '/50\.00% in/ {print $(NF-1); exit}')"
  p99="$(echo "$raw" | awk '/99\.00% in/ {print $(NF-1); exit}')"
  success="$(echo "$raw" | awk '/Success rate:/ {print $NF; exit}')"
  rps="${rps:-0}"
  p50="${p50:-0}"
  p99="${p99:-0}"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$round" "$name" "$path" "$rps" "$p50" "$p99" "$success"
}

echo "==> load PROFILE=$PROFILE DURATION=$DURATION CONCURRENCY=$CONCURRENCY ROUNDS=$ROUNDS workers=$TOKIO_WORKER_THREADS (median of rounds; rotate fw order)"
RAW_TSV="$RESULTS/load_raw.tsv"
{
  echo -e "round\tframework\tpath\trps\tp50\tp99\tsuccess"
  for ((round = 1; round <= ROUNDS; round++)); do
    # Rotate who runs first so cold-CPU tax is not always on sova.
    offset=$(( (round - 1) % 3 ))
    echo "  round $round / $ROUNDS (first fw offset=$offset)" >&2
    for path in "${PATHS[@]}"; do
      echo "    GET $path" >&2
      for ((k = 0; k < 3; k++)); do
        idx=$(( (offset + k) % 3 ))
        oha_one "${FWS[$idx]}" "${PORTS[$idx]}" "$path" "$round"
      done
    done
    echo "    POST /api/echo" >&2
    for ((k = 0; k < 3; k++)); do
      idx=$(( (offset + k) % 3 ))
      oha_one "${FWS[$idx]}" "${PORTS[$idx]}" "/api/echo" "$round" POST "$ROOT/fixtures/echo.json"
    done
  done
} | tee "$RAW_TSV"

echo "==> summarizing (median across rounds)"
python3 - "$RAW_TSV" "$RESULTS" "$DURATION" "$CONCURRENCY" "$TOKIO_WORKER_THREADS" "$REPO" "$UPDATE_BASELINE" "$REGRESSION_RPS_PCT" "$REGRESSION_P99_PCT" "$PROFILE" "$ROUNDS" "$WARMUP" <<'PY'
import csv, json, os, sys, datetime as dt, statistics
from pathlib import Path

(
    load_tsv, results_dir, duration, concurrency, workers, repo,
    update_bl, rps_pct, p99_pct, profile, rounds, warmup,
) = sys.argv[1:]
update_bl = update_bl == "1"
rps_pct = float(rps_pct)
p99_pct = float(p99_pct)
rounds = int(rounds)
results_dir = Path(results_dir)
repo = Path(repo)

raw = []
with open(load_tsv, newline="") as f:
    for row in csv.DictReader(f, delimiter="\t"):
        raw.append({
            "round": int(row["round"]),
            "framework": row["framework"],
            "path": row["path"],
            "rps": float(row["rps"]) if row["rps"] else 0.0,
            "p50_ms": float(row["p50"]) if row["p50"] else 0.0,
            "p99_ms": float(row["p99"]) if row["p99"] else 0.0,
            "success": row.get("success") or "",
        })

def median(xs):
    return float(statistics.median(xs)) if xs else 0.0

# Aggregate per (framework, path)
from collections import defaultdict
groups = defaultdict(list)
for r in raw:
    groups[(r["framework"], r["path"])].append(r)

rows = []
for (fw, path), items in sorted(groups.items(), key=lambda x: (x[0][0], x[0][1])):
    rps_l = [i["rps"] for i in items]
    p50_l = [i["p50_ms"] for i in items]
    p99_l = [i["p99_ms"] for i in items]
    rows.append({
        "framework": fw,
        "path": path,
        "rps": median(rps_l),
        "p50_ms": median(p50_l),
        "p99_ms": median(p99_l),
        "rps_min": min(rps_l),
        "rps_max": max(rps_l),
        "rps_spread_pct": ((max(rps_l) - min(rps_l)) / median(rps_l) * 100) if median(rps_l) else 0.0,
        "samples": len(rps_l),
        "success": items[-1]["success"],
    })

by_fw = defaultdict(list)
for r in rows:
    by_fw[r["framework"]].append(r)

summary = {
    "captured_at": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "host": os.uname().nodename if hasattr(os, "uname") else "",
    "profile": profile,
    "build": "release",
    "duration": duration,
    "concurrency": int(concurrency),
    "tokio_worker_threads": int(workers),
    "rounds": rounds,
    "warmup": warmup,
    "aggregate": "median",
    "paths": sorted({r["path"] for r in rows}),
    "frameworks": {},
    "rows": rows,
    "raw": raw,
}

for fw, items in by_fw.items():
    home = next((i for i in items if i["path"] == "/"), items[0])
    echo = next((i for i in items if i["path"] == "/api/echo"), None)
    spreads = [i["rps_spread_pct"] for i in items]
    summary["frameworks"][fw] = {
        "home_rps": home["rps"],
        "home_p50_ms": home["p50_ms"],
        "home_p99_ms": home["p99_ms"],
        "home_rps_min": home["rps_min"],
        "home_rps_max": home["rps_max"],
        "echo_rps": echo["rps"] if echo else 0.0,
        "echo_p99_ms": echo["p99_ms"] if echo else 0.0,
        "mean_rps": sum(i["rps"] for i in items) / len(items),
        "mean_p99_ms": sum(i["p99_ms"] for i in items) / len(items),
        "max_path_spread_pct": max(spreads) if spreads else 0.0,
    }

latest = results_dir / "latest.json"
latest.write_text(json.dumps(summary, indent=2) + "\n")

# Also write compact TSV of medians for quick grepping
with open(results_dir / "load.tsv", "w") as f:
    f.write("framework\tpath\trps\tp50\tp99\trps_min\trps_max\tspread_pct\n")
    for r in rows:
        f.write(
            f"{r['framework']}\t{r['path']}\t{r['rps']:.4f}\t{r['p50_ms']:.4f}\t"
            f"{r['p99_ms']:.4f}\t{r['rps_min']:.4f}\t{r['rps_max']:.4f}\t{r['rps_spread_pct']:.2f}\n"
        )

baseline_path = results_dir / "baseline.json"
regressions = []
if baseline_path.exists() and not update_bl:
    baseline = json.loads(baseline_path.read_text())
    for fw, cur in summary["frameworks"].items():
        base = baseline.get("frameworks", {}).get(fw)
        if not base:
            continue
        if base["home_rps"] > 0:
            drop = (base["home_rps"] - cur["home_rps"]) / base["home_rps"] * 100
            if drop > rps_pct:
                regressions.append(
                    f"{fw} home RPS dropped {drop:.1f}% ({base['home_rps']:.0f} → {cur['home_rps']:.0f})"
                )
        if base["home_p99_ms"] > 0:
            rise = (cur["home_p99_ms"] - base["home_p99_ms"]) / base["home_p99_ms"] * 100
            if rise > p99_pct:
                regressions.append(
                    f"{fw} home p99 rose {rise:.1f}% ({base['home_p99_ms']:.3f} → {cur['home_p99_ms']:.3f} ms)"
                )

if update_bl or not baseline_path.exists():
    baseline_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(f"baseline written → {baseline_path}")

md_lines = []
md_lines.append("# Performance")
md_lines.append("")
md_lines.append("![Performance](/banners/performance.svg)")
md_lines.append("")
md_lines.append("Sova vs Axum vs Actix-web on an **identical multi-page fixture site** (same HTML/JSON bodies, verified SHA-256), including a realistic **POST /api/echo** JSON path.")
md_lines.append("")
md_lines.append("## Methodology")
md_lines.append("")
md_lines.append("- Stand: `bench/stand/` — shared fixtures, three **release** servers (`stand_sova`, `stand_axum`, `stand_actix`).")
md_lines.append("- Workspace `[profile.release]` uses thin LTO (`codegen-units = 1`) — production-shaped binaries, not `dev`.")
md_lines.append("- Bodies must match **byte-for-byte** across frameworks before load runs.")
md_lines.append("- Load tool: [oha](https://github.com/hatoo/oha).")
md_lines.append(f"- Stability: oha warm-up `{warmup}` at full concurrency; **{rounds} round(s)** with rotating framework order; reported numbers are the **median** RPS (min/max kept in JSON for spread).")
md_lines.append(f"- This capture: profile `{profile}`, duration `{duration}`, concurrency `{concurrency}`, `TOKIO_WORKER_THREADS={workers}`.")
md_lines.append(f"- Captured at `{summary['captured_at']}` on `{summary['host']}`.")
md_lines.append("")
md_lines.append("Pages: `/`, `/about`, `/blog`, `/blog/hello`, `/contact`, `/api/health`, `POST /api/echo`.")
md_lines.append("")
md_lines.append("## Latest results — `GET /` (median)")
md_lines.append("")
md_lines.append("| Framework | Req/s | min–max | p50 (ms) | p99 (ms) |")
md_lines.append("|-----------|-------|---------|----------|----------|")
for fw in ("sova", "axum", "actix"):
    f = summary["frameworks"][fw]
    md_lines.append(
        f"| {fw} | {f['home_rps']:.0f} | {f['home_rps_min']:.0f}–{f['home_rps_max']:.0f} | "
        f"{f['home_p50_ms']:.3f} | {f['home_p99_ms']:.3f} |"
    )
md_lines.append("")
md_lines.append("## Latest results — `POST /api/echo` (median)")
md_lines.append("")
md_lines.append("| Framework | Req/s | p99 (ms) |")
md_lines.append("|-----------|-------|----------|")
for fw in ("sova", "axum", "actix"):
    f = summary["frameworks"][fw]
    md_lines.append(f"| {fw} | {f['echo_rps']:.0f} | {f['echo_p99_ms']:.3f} |")
md_lines.append("")
md_lines.append("## Latest results — mean across all paths (of medians)")
md_lines.append("")
md_lines.append("| Framework | Mean Req/s | Mean p99 (ms) | max path spread % |")
md_lines.append("|-----------|------------|---------------|-------------------|")
for fw in ("sova", "axum", "actix"):
    f = summary["frameworks"][fw]
    md_lines.append(
        f"| {fw} | {f['mean_rps']:.0f} | {f['mean_p99_ms']:.3f} | {f['max_path_spread_pct']:.1f}% |"
    )
md_lines.append("")
md_lines.append("## Per-path detail (median)")
md_lines.append("")
md_lines.append("| Framework | Path | Req Req/s | min–max | spread % | p50 | p99 |")
md_lines.append("|-----------|------|------------|---------|----------|-----|-----|")
for r in rows:
    md_lines.append(
        f"| {r['framework']} | `{r['path']}` | {r['rps']:.0f} | "
        f"{r['rps_min']:.0f}–{r['rps_max']:.0f} | {r['rps_spread_pct']:.1f}% | "
        f"{r['p50_ms']:.3f} | {r['p99_ms']:.3f} |"
    )
md_lines.append("")
md_lines.append("## Re-run / regression gate")
md_lines.append("")
md_lines.append("```bash")
md_lines.append("./bench/stand/run.sh                     # deep: 15s × 3 rounds, warm-up, median")
md_lines.append("PROFILE=quick ./bench/stand/run.sh      # smoke")
md_lines.append("./bench/stand/run.sh --update-baseline")
md_lines.append("ROUNDS=5 DURATION=20s ./bench/stand/run.sh")
md_lines.append("```")
md_lines.append("")
md_lines.append(f"Regression thresholds (vs `bench/stand/results/baseline.json`): home RPS drop > {rps_pct:.0f}% or p99 rise > {p99_pct:.0f}% fails the script.")
md_lines.append("")
md_lines.append("Machine-sensitive: compare relative rankings and deltas, not absolute RPS across laptops.")
md_lines.append("")

docs_page = repo / "docs" / "guide" / "performance.md"
docs_page.write_text("\n".join(md_lines) + "\n")
print(f"docs updated → {docs_page}")

(results_dir / "latest.md").write_text("\n".join(md_lines) + "\n")

# Stability check: warn if any path spread is wild
wild = [r for r in rows if r["rps_spread_pct"] > 15]
if wild:
    print("NOTE: high round-to-round spread (>15%) on:", file=sys.stderr)
    for r in wild:
        print(
            f" - {r['framework']} {r['path']}: {r['rps_spread_pct']:.1f}% "
            f"({r['rps_min']:.0f}–{r['rps_max']:.0f})",
            file=sys.stderr,
        )

if regressions:
    print("REGRESSION:", file=sys.stderr)
    for line in regressions:
        print(" -", line, file=sys.stderr)
    sys.exit(2)

print("ok")
PY

echo "==> done"
