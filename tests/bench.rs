// Integration tests for benchmarks - these will show up in IntelliJ's test sidebar
use dbex::DBex;
use std::time::{Duration, Instant};
use rand::Rng;

struct BenchResult {
    operation: String,
    count: usize,
    total_time: Duration,
    ops_per_sec: f64,
    avg_latency_us: f64,
}

impl BenchResult {
    fn print(&self) {
        println!(
            "{:<20} {:>10} ops in {:>10.2?} ({:>12.0} ops/sec, {:>8.2} µs/op)",
            self.operation,
            self.count,
            self.total_time,
            self.ops_per_sec,
            self.avg_latency_us,
        );
    }
}

fn zipfian_key(rng: &mut impl Rng, n: usize) -> usize {
    // Simple approximation: 80% of accesses hit 20% of keys
    if rng.random_bool(0.8) {
        rng.random_range(0..n / 5)  // hot keys
    } else {
        rng.random_range(0..n)       // all keys
    }
}

fn bench_sequential_writes(db: &mut DBex, num_keys: usize, value_size: usize) -> BenchResult {
    let value = vec![0xABu8; value_size];

    let start = Instant::now();
    for i in 0..num_keys {
        let key = i.to_be_bytes();
        db.insert(&key, &value);
    }
    db.flush();
    let total_time = start.elapsed();

    BenchResult {
        operation: "sequential_write".into(),
        count: num_keys,
        total_time,
        ops_per_sec: num_keys as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_keys as f64,
    }
}

fn bench_random_reads(db: &mut DBex, num_reads: usize, key_space: usize) -> BenchResult {
    let mut rng = rand::rng();

    let start = Instant::now();
    for _ in 0..num_reads {
        let i = rng.random_range(0..key_space);
        let key = i.to_be_bytes();
        let _ = db.find(&key);
    }
    let total_time = start.elapsed();

    BenchResult {
        operation: "random_read".into(),
        count: num_reads,
        total_time,
        ops_per_sec: num_reads as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_reads as f64,
    }
}

fn bench_sequential_reads(db: &mut DBex, num_reads: usize) -> BenchResult {
    let start = Instant::now();
    for i in 0..num_reads {
        let key = i.to_be_bytes();
        let _ = db.find(&key);
    }
    let total_time = start.elapsed();

    BenchResult {
        operation: "sequential_read".into(),
        count: num_reads,
        total_time,
        ops_per_sec: num_reads as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_reads as f64,
    }
}

fn bench_zipfian_reads(db: &mut DBex, num_reads: usize, key_space: usize) -> BenchResult {
    let mut rng = rand::rng();

    let start = Instant::now();
    for _ in 0..num_reads {
        let i = zipfian_key(&mut rng, key_space);
        let key = i.to_be_bytes();
        let _ = db.find(&key);
    }
    let total_time = start.elapsed();

    BenchResult {
        operation: "zipfian_read".into(),
        count: num_reads,
        total_time,
        ops_per_sec: num_reads as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_reads as f64,
    }
}

fn run_benchmark(name: &str, num_keys: usize, value_size: usize, num_reads: usize) {
    // Clean up any previous test file
    let db_path = format!("bench_{}.db", name);
    let _ = std::fs::remove_file(&db_path);

    let mut db = DBex::new(&db_path);

    let data_size_mb = (num_keys * (8 + value_size)) as f64 / 1_000_000.0;

    println!("\n{}", format!("{:=<60}", ""));
    println!("Benchmark: {}", name);
    println!("Keys: {}, Value size: {} bytes, Total data: {:.1} MB", num_keys, value_size, data_size_mb);
    println!("\n{}", format!("{:=<60}", ""));

    bench_sequential_writes(&mut db, num_keys, value_size).print();
    bench_sequential_reads(&mut db, num_reads.min(num_keys)).print();
    bench_random_reads(&mut db, num_reads, num_keys).print();
    bench_zipfian_reads(&mut db, num_reads, num_keys).print();

    // Cleanup
    drop(db);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn bench_tiny() {
    run_benchmark("tiny", 1_000, 100, 10_000);
}

#[test]
fn bench_small() {
    run_benchmark("small", 100_000, 100, 10_000);
}

#[test]
fn bench_medium() {
    run_benchmark("medium", 1_000_000, 100, 10_000);
}

#[test]
fn bench_large() {
    run_benchmark("large", 1_000_000, 1_000, 10_000);
}

#[test]
fn bench_large_heavy_reads() {
    run_benchmark("large_heavy_reads", 1_000_000, 8_000, 100_000);
}
