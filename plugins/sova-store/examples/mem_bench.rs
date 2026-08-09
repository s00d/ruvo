//! Temporary release microbench for MemoryStore. Do not commit.
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use sova_store::{KvStore, MemoryStore};

const THREADS: usize = 8;
const OPS_PER_TASK: usize = 50_000;
const KEY_POOL: usize = 10_000;
const GET_PCT: u64 = 80;

struct Stats {
    latencies_ns: Vec<u64>,
    wall: Duration,
    total_ops: usize,
}

fn percentile(sorted: &[u64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)] as f64
}

async fn run_bench(label: &str, store: Arc<dyn KvStore>) -> Stats {
    // Warm / seed a bit so gets can hit.
    for i in 0..KEY_POOL {
        let k = format!("k{i}");
        store.set(&k, Bytes::from_static(b"v"), None).await;
    }

    let wall_start = Instant::now();
    let mut handles = Vec::with_capacity(THREADS);

    for t in 0..THREADS {
        let store = Arc::clone(&store);
        handles.push(tokio::spawn(async move {
            let mut lats = Vec::with_capacity(OPS_PER_TASK);
            let mut rng = t as u64 * 0x9E37_79B9_7F4A_7C15 + 1;
            for i in 0..OPS_PER_TASK {
                rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1);
                let key_i = (rng as usize) % KEY_POOL;
                let key = format!("k{key_i}");
                let is_get = ((rng >> 33) % 100) < GET_PCT;

                let t0 = Instant::now();
                if is_get {
                    let _ = store.get(&key).await;
                } else {
                    store
                        .set(&key, Bytes::from(format!("v{i}")), None)
                        .await;
                }
                lats.push(t0.elapsed().as_nanos() as u64);
            }
            lats
        }));
    }

    let mut all = Vec::with_capacity(THREADS * OPS_PER_TASK);
    for h in handles {
        all.extend(h.await.expect("task"));
    }
    let wall = wall_start.elapsed();
    let total_ops = all.len();
    all.sort_unstable();

    let ops_sec = total_ops as f64 / wall.as_secs_f64();
    let p50 = percentile(&all, 50.0) / 1000.0;
    let p99 = percentile(&all, 99.0) / 1000.0;

    println!(
        "{label}: ops/sec={ops_sec:.0}  p50={p50:.2} µs  p99={p99:.2} µs  wall={:.3} s  (ops={total_ops})",
        wall.as_secs_f64()
    );

    Stats {
        latencies_ns: all,
        wall,
        total_ops,
    }
}

fn print_row(name: &str, s: &Stats) {
    let ops_sec = s.total_ops as f64 / s.wall.as_secs_f64();
    let p50 = percentile(&s.latencies_ns, 50.0) / 1000.0;
    let p99 = percentile(&s.latencies_ns, 99.0) / 1000.0;
    println!(
        "| {name} | {ops_sec:.0} | {p50:.2} µs | {p99:.2} µs | {:.3} s |",
        s.wall.as_secs_f64()
    );
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    println!("MemoryStore release bench: {THREADS} tasks × {OPS_PER_TASK} ops, key pool {KEY_POOL}, {GET_PCT}% get");
    println!();

    let a = run_bench(
        "A default shards",
        Arc::new(MemoryStore::new()) as Arc<dyn KvStore>,
    )
    .await;
    let b = run_bench(
        "B shards(1)",
        Arc::new(MemoryStore::with_shards(1)) as Arc<dyn KvStore>,
    )
    .await;

    println!();
    println!("| Scenario | ops/sec | p50 | p99 | wall |");
    println!("|---|---:|---:|---:|---:|");
    print_row("A default shards", &a);
    print_row("B shards(1)", &b);
}
