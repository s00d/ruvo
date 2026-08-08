#!/usr/bin/env bash
# Framework comparison stand: byte-identical bodies + oha load + regression gate.
#
# Usage:
#   ./bench/stand/run.sh              # verify + load, write results
#   ./bench/stand/run.sh --update-baseline
#   DURATION=15s CONCURRENCY=100 ./bench/stand/run.sh
#
# Requires: oha, curl, python3, cargo
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
RESULTS="$ROOT/results"
DURATION="${DURATION:-10s}"
CONCURRENCY="${CONCURRENCY:-50}"
TOKIO_WORKER_THREADS="${TOKIO_WORKER_THREADS:-4}"
UPDATE_BASELINE=0
REGRESSION_RPS_PCT="${REGRESSION_RPS_PCT:-15}"   # fail if RPS drops more than this %
REGRESSION_P99_PCT="${REGRESSION_P99_PCT:-40}"   # fail if p99 rises more than this %

for arg in "$@"; do
  case "$arg" in
    --update-baseline) UPDATE_BASELINE=1 ;;
    -h|--help)
      sed -n '2,12p' "$0"
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

echo "==> building stand binaries"
TOKIO_WORKER_THREADS="$TOKIO_WORKER_THREADS" cargo build --release -q \
  -p stand_ruvo -p stand_axum -p stand_actix

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

start_one ruvo 9101 stand_ruvo
start_one axum 9102 stand_axum
start_one actix 9103 stand_actix

PATHS=(/ /about /blog /blog/hello /contact /api/health)

echo "==> verifying byte-identical response bodies"
VERIFY_JSON="$RESULTS/verify.json"
python3 - "$VERIFY_JSON" <<'PY'
import hashlib, json, sys, urllib.request

out_path = sys.argv[1]
ports = {"ruvo": 9101, "axum": 9102, "actix": 9103}
paths = ["/", "/about", "/blog", "/blog/hello", "/contact", "/api/health"]

def fetch(port, path):
    with urllib.request.urlopen(f"http://127.0.0.1:{port}{path}", timeout=5) as r:
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
        # show first differing pair
        names = list(bodies)
        a, b = names[0], names[1]
        entry["diff"] = f"{a} vs {b}: {hashes[a]} != {hashes[b]}"
    report["paths"][path] = entry

with open(out_path, "w") as f:
    json.dump(report, f, indent=2)
    f.write("\n")

if not report["ok"]:
    print("BODY MISMATCH", json.dumps(report, indent=2))
    sys.exit(1)
print("bodies match for all paths across ruvo/axum/actix")
PY

oha_one() {
  local name="$1" port="$2" path="$3"
  local url="http://127.0.0.1:${port}${path}"
  local raw
  raw="$(oha -z "$DURATION" -c "$CONCURRENCY" --no-tui "$url" 2>&1)" || true
  local rps p50 p99 success
  rps="$(echo "$raw" | awk '/Requests\/sec:/ {print $NF; exit}')"
  p50="$(echo "$raw" | awk '/50\.00% in/ {print $(NF-1); exit}')"
  p99="$(echo "$raw" | awk '/99\.00% in/ {print $(NF-1); exit}')"
  success="$(echo "$raw" | awk '/Success rate:/ {print $NF; exit}')"
  # fallbacks
  rps="${rps:-0}"
  p50="${p50:-0}"
  p99="${p99:-0}"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$path" "$rps" "$p50" "$p99" "$success"
}

echo "==> load test (DURATION=$DURATION CONCURRENCY=$CONCURRENCY workers=$TOKIO_WORKER_THREADS)"
LOAD_TSV="$RESULTS/load.tsv"
{
  echo -e "framework\tpath\trps\tp50\tp99\tsuccess"
  for path in "${PATHS[@]}"; do
    echo "  load / path=$path" >&2
    oha_one ruvo 9101 "$path"
    oha_one axum 9102 "$path"
    oha_one actix 9103 "$path"
  done
} | tee "$LOAD_TSV"

echo "==> summarizing"
python3 - "$LOAD_TSV" "$RESULTS" "$DURATION" "$CONCURRENCY" "$TOKIO_WORKER_THREADS" "$REPO" "$UPDATE_BASELINE" "$REGRESSION_RPS_PCT" "$REGRESSION_P99_PCT" <<'PY'
import csv, json, os, sys, datetime as dt
from pathlib import Path

load_tsv, results_dir, duration, concurrency, workers, repo, update_bl, rps_pct, p99_pct = sys.argv[1:]
update_bl = update_bl == "1"
rps_pct = float(rps_pct)
p99_pct = float(p99_pct)
results_dir = Path(results_dir)
repo = Path(repo)

rows = []
with open(load_tsv, newline="") as f:
    for row in csv.DictReader(f, delimiter="\t"):
        rows.append({
            "framework": row["framework"],
            "path": row["path"],
            "rps": float(row["rps"]) if row["rps"] else 0.0,
            "p50_ms": float(row["p50"]) if row["p50"] else 0.0,
            "p99_ms": float(row["p99"]) if row["p99"] else 0.0,
            "success": row.get("success") or "",
        })

# Aggregate mean RPS across paths per framework (home-weighted primary = "/")
by_fw = {}
for r in rows:
    by_fw.setdefault(r["framework"], []).append(r)

summary = {
    "captured_at": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "host": os.uname().nodename if hasattr(os, "uname") else "",
    "duration": duration,
    "concurrency": int(concurrency),
    "tokio_worker_threads": int(workers),
    "paths": sorted({r["path"] for r in rows}),
    "frameworks": {},
    "rows": rows,
}

for fw, items in by_fw.items():
    home = next((i for i in items if i["path"] == "/"), items[0])
    summary["frameworks"][fw] = {
        "home_rps": home["rps"],
        "home_p50_ms": home["p50_ms"],
        "home_p99_ms": home["p99_ms"],
        "mean_rps": sum(i["rps"] for i in items) / len(items),
        "mean_p99_ms": sum(i["p99_ms"] for i in items) / len(items),
    }

latest = results_dir / "latest.json"
latest.write_text(json.dumps(summary, indent=2) + "\n")

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

# Markdown for docs
md_lines = []
md_lines.append("# Performance")
md_lines.append("")
md_lines.append("![Performance](/banners/performance.svg)")
md_lines.append("")
md_lines.append("Ruvo vs Axum vs Actix-web on an **identical multi-page fixture site** (same HTML/JSON bodies, verified SHA-256).")
md_lines.append("")
md_lines.append("## Methodology")
md_lines.append("")
md_lines.append("- Stand: `bench/stand/` — shared fixtures in `fixtures/`, three minimal servers (`stand_ruvo`, `stand_axum`, `stand_actix`).")
md_lines.append("- Bodies must match **byte-for-byte** across frameworks before load runs (`run.sh` aborts on mismatch).")
md_lines.append("- Load tool: [oha](https://github.com/hatoo/oha).")
md_lines.append(f"- This capture: duration `{duration}`, concurrency `{concurrency}`, `TOKIO_WORKER_THREADS={workers}`.")
md_lines.append(f"- Captured at `{summary['captured_at']}` on `{summary['host']}`.")
md_lines.append("")
md_lines.append("Pages: `/`, `/about`, `/blog`, `/blog/hello`, `/contact`, `/api/health`.")
md_lines.append("")
md_lines.append("## Latest results — `GET /`")
md_lines.append("")
md_lines.append("| Framework | Req/s | p50 (ms) | p99 (ms) |")
md_lines.append("|-----------|-------|----------|----------|")
for fw in ("ruvo", "axum", "actix"):
    f = summary["frameworks"][fw]
    md_lines.append(
        f"| {fw} | {f['home_rps']:.0f} | {f['home_p50_ms']:.3f} | {f['home_p99_ms']:.3f} |"
    )
md_lines.append("")
md_lines.append("## Latest results — mean across all paths")
md_lines.append("")
md_lines.append("| Framework | Mean Req/s | Mean p99 (ms) |")
md_lines.append("|-----------|------------|---------------|")
for fw in ("ruvo", "axum", "actix"):
    f = summary["frameworks"][fw]
    md_lines.append(f"| {fw} | {f['mean_rps']:.0f} | {f['mean_p99_ms']:.3f} |")
md_lines.append("")
md_lines.append("## Per-path detail")
md_lines.append("")
md_lines.append("| Framework | Path | Req/s | p50 (ms) | p99 (ms) |")
md_lines.append("|-----------|------|-------|----------|----------|")
for r in rows:
    md_lines.append(
        f"| {r['framework']} | `{r['path']}` | {r['rps']:.0f} | {r['p50_ms']:.3f} | {r['p99_ms']:.3f} |"
    )
md_lines.append("")
md_lines.append("## Re-run / regression gate")
md_lines.append("")
md_lines.append("```bash")
md_lines.append("./bench/stand/run.sh")
md_lines.append("./bench/stand/run.sh --update-baseline   # after intentional perf changes")
md_lines.append("DURATION=15s CONCURRENCY=100 ./bench/stand/run.sh")
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

if regressions:
    print("REGRESSION:", file=sys.stderr)
    for line in regressions:
        print(" -", line, file=sys.stderr)
    sys.exit(2)

print("ok")
PY

echo "==> done"
