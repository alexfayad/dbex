// Integration tests for benchmarks - these will show up in IntelliJ's test sidebar
mod test_db;
use test_db::TestDb;

use dbex::DBex;
use std::time::{Duration, Instant, SystemTime};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use rand::Rng;
use sysinfo::System;
use std::process;

// Cached bench directory - created once per test run, reused by all benchmarks
static BENCH_DIR: OnceLock<PathBuf> = OnceLock::new();

// Get versioned bench run directory (creates once, then reuses)
fn get_bench_dir() -> PathBuf {
    BENCH_DIR.get_or_init(|| {
        let version = env!("CARGO_PKG_VERSION");
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let dir = PathBuf::from(format!("bench_runs/v{}__{}", version, timestamp));
        fs::create_dir_all(&dir).ok();
        dir
    }).clone()
}

struct BenchResult {
    operation: String,
    count: usize,
    total_time: Duration,
    ops_per_sec: f64,
    avg_latency_us: f64,
    throughput_mb_s: Option<f64>,  // MB/s if applicable
}

impl BenchResult {
    fn print(&self) {
        if let Some(mb_s) = self.throughput_mb_s {
            println!(
                "{:<20} {:>10} ops in {:>10.2?} ({:>12.0} ops/sec, {:>8.2} µs/op, {:>8.1} MB/s)",
                self.operation,
                self.count,
                self.total_time,
                self.ops_per_sec,
                self.avg_latency_us,
                mb_s,
            );
        } else {
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
}

struct MemoryTracker {
    system: System,
    samples: Vec<f64>,
}

impl MemoryTracker {
    fn new() -> Self {
        MemoryTracker {
            system: System::new(),  // Only allocate the System, don't load all processes
            samples: Vec::new(),
        }
    }

    fn sample(&mut self) {
        let pid = sysinfo::Pid::from_u32(process::id());

        // Refresh only our specific process
        self.system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);

        if let Some(process) = self.system.process(pid) {
            let mem_mb = process.memory() as f64 / 1_048_576.0;  // Convert to MB
            self.samples.push(mem_mb);
        }
    }

    fn peak_mb(&self) -> f64 {
        self.samples.iter().copied().fold(0.0, f64::max)
    }

    fn avg_mb(&self) -> f64 {
        if self.samples.is_empty() {
            0.0
        } else {
            self.samples.iter().sum::<f64>() / self.samples.len() as f64
        }
    }

    fn samples(&self) -> Vec<f64> {
        self.samples.clone()
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
        let key = i.to_be_bytes().to_vec();
        let value = value.clone();
        db.insert(key, value);
    }
    db.flush();
    let total_time = start.elapsed();

    let total_bytes = num_keys * (8 + value_size);  // 8 byte keys + value_size
    let throughput_mb_s = (total_bytes as f64 / 1_000_000.0) / total_time.as_secs_f64();

    BenchResult {
        operation: "sequential_write".into(),
        count: num_keys,
        total_time,
        ops_per_sec: num_keys as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_keys as f64,
        throughput_mb_s: Some(throughput_mb_s),
    }
}

fn bench_random_reads(db: &mut DBex, num_reads: usize, key_space: usize, value_size: usize) -> BenchResult {
    let mut rng = rand::rng();

    let start = Instant::now();
    for _ in 0..num_reads {
        let idx = rng.random_range(0..key_space);
        let key = idx.to_be_bytes();
        let _ = db.find(&key);
    }
    let total_time = start.elapsed();

    let total_bytes = num_reads * (8 + value_size);
    let throughput_mb_s = (total_bytes as f64 / 1_000_000.0) / total_time.as_secs_f64();

    BenchResult {
        operation: "random_read".into(),
        count: num_reads,
        total_time,
        ops_per_sec: num_reads as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_reads as f64,
        throughput_mb_s: Some(throughput_mb_s),
    }
}

fn bench_sequential_reads(db: &mut DBex, num_reads: usize, value_size: usize) -> BenchResult {
    let start = Instant::now();
    for i in 0..num_reads {
        let key = i.to_be_bytes();
        let _ = db.find(&key);
    }
    let total_time = start.elapsed();

    let total_bytes = num_reads * (8 + value_size);
    let throughput_mb_s = (total_bytes as f64 / 1_000_000.0) / total_time.as_secs_f64();

    BenchResult {
        operation: "sequential_read".into(),
        count: num_reads,
        total_time,
        ops_per_sec: num_reads as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_reads as f64,
        throughput_mb_s: Some(throughput_mb_s),
    }
}

fn bench_zipfian_reads(db: &mut DBex, num_reads: usize, key_space: usize, value_size: usize) -> BenchResult {
    let mut rng = rand::rng();

    let start = Instant::now();
    for _ in 0..num_reads {
        let idx = zipfian_key(&mut rng, key_space);
        let key = idx.to_be_bytes();
        let _ = db.find(&key);
    }
    let total_time = start.elapsed();

    let total_bytes = num_reads * (8 + value_size);
    let throughput_mb_s = (total_bytes as f64 / 1_000_000.0) / total_time.as_secs_f64();

    BenchResult {
        operation: "zipfian_read".into(),
        count: num_reads,
        total_time,
        ops_per_sec: num_reads as f64 / total_time.as_secs_f64(),
        avg_latency_us: total_time.as_micros() as f64 / num_reads as f64,
        throughput_mb_s: Some(throughput_mb_s),
    }
}

fn run_benchmark(name: &str, num_keys: usize, value_size: usize, num_reads: usize) {
    // Create versioned directory for this benchmark run
    let bench_dir = get_bench_dir();

    let mut test_db = TestDb::new();
    let db = test_db.db();

    let data_size_mb = (num_keys * (8 + value_size)) as f64 / 1_000_000.0;

    // Collect results
    let mut output = String::new();
    output.push_str(&format!("\n{}\n", "=".repeat(60)));
    output.push_str(&format!("Benchmark: {}\n", name));
    output.push_str(&format!("Keys: {}, Value size: {} bytes, Total data: {:.1} MB\n", num_keys, value_size, data_size_mb));
    output.push_str(&format!("\n{}\n", "=".repeat(60)));

    println!("{}", output);

    let write_result = bench_sequential_writes(db, num_keys, value_size);
    let seq_read_result = bench_sequential_reads(db, num_reads.min(num_keys), value_size);
    let random_read_result = bench_random_reads(db, num_reads, num_keys, value_size);
    let zipfian_result = bench_zipfian_reads(db, num_reads, num_keys, value_size);

    let total_ss_tables = &format!("Total SSTables: {}", db.num_of_ss_tables());

    write_result.print();
    seq_read_result.print();
    random_read_result.print();
    zipfian_result.print();
    println!("{}", total_ss_tables);

    // Append results to output
    output.push_str(&format_result(&write_result));
    output.push_str(&format_result(&seq_read_result));
    output.push_str(&format_result(&random_read_result));
    output.push_str(&format_result(&zipfian_result));
    output.push_str("\n");
    output.push_str(total_ss_tables);

    // Save results to file
    let results_file = bench_dir.join(format!("{}.txt", name));
    fs::write(&results_file, output).ok();

    // Cleanup database files
    db.purge();

    println!("Results saved to: {}", results_file.display());
}

fn format_result(result: &BenchResult) -> String {
    if let Some(mb_s) = result.throughput_mb_s {
        format!(
            "{:<20} {:>10} ops in {:>10.2?} ({:>12.0} ops/sec, {:>8.2} µs/op, {:>8.1} MB/s)\n",
            result.operation,
            result.count,
            result.total_time,
            result.ops_per_sec,
            result.avg_latency_us,
            mb_s,
        )
    } else {
        format!(
            "{:<20} {:>10} ops in {:>10.2?} ({:>12.0} ops/sec, {:>8.2} µs/op)\n",
            result.operation,
            result.count,
            result.total_time,
            result.ops_per_sec,
            result.avg_latency_us,
        )
    }
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
    run_benchmark("large_heavy_reads", 1_000_000, 1_000, 100_000);
}

#[test]
fn bench_large_heavy_reads_and_writes() {
    run_benchmark("large_heavy_reads_and_writes", 1_000_000, 8_000, 100_000);
}

#[test]
fn bench_huge() {
    run_benchmark("huge", 10_000_000, 1_000, 10_000);
}

// Dedicated memory stress test with full tracking
#[test]
fn bench_memory_stress() {
    let bench_dir = get_bench_dir();
    let mut test_db = TestDb::new();
    let db = test_db.db();

    let num_keys = 5_000_000;
    let value_size = 2_000;  // 2KB values
    let num_reads = 50_000;

    println!("\n{}", "=".repeat(60));
    println!("Memory Stress Test");
    println!("Keys: {}, Value size: {} bytes, Total data: {:.1} MB",
             num_keys, value_size, (num_keys * (8 + value_size)) as f64 / 1_000_000.0);
    println!("{}\n", "=".repeat(60));

    let mut mem_tracker = MemoryTracker::new();
    let value = vec![0xABu8; value_size];

    // Phase 1: Write with memory tracking
    println!("Phase 1: Writing {} keys with memory tracking...", num_keys);
    mem_tracker.sample();
    let write_start = Instant::now();

    for i in 0..num_keys {
        let key = i.to_be_bytes().to_vec();
        db.insert(key, value.clone());

        if i % 50000 == 0 {
            mem_tracker.sample();
        }
    }
    db.flush();
    mem_tracker.sample();
    let write_time = write_start.elapsed();

    let write_samples = mem_tracker.samples();
    let write_peak = mem_tracker.peak_mb();
    let write_avg = mem_tracker.avg_mb();

    println!("Write complete: {:.2?} ({:.0} ops/sec, mem: {:.1}/{:.1} MB)",
             write_time,
             num_keys as f64 / write_time.as_secs_f64(),
             write_avg,
             write_peak);
    println!("Total SSTables: {}\n", db.num_of_ss_tables());

    // Phase 2: Random reads with memory tracking
    println!("Phase 2: Random reads with memory tracking...");
    mem_tracker = MemoryTracker::new();
    mem_tracker.sample();

    let mut rng = rand::rng();
    let read_start = Instant::now();

    for i in 0..num_reads {
        let idx = rng.random_range(0..num_keys);
        let key = idx.to_be_bytes();
        let _ = db.find(&key);

        if i % 5000 == 0 {
            mem_tracker.sample();
        }
    }
    let read_time = read_start.elapsed();

    let read_samples = mem_tracker.samples();
    let read_peak = mem_tracker.peak_mb();
    let read_avg = mem_tracker.avg_mb();

    println!("Reads complete: {:.2?} ({:.0} ops/sec, mem: {:.1}/{:.1} MB)\n",
             read_time,
             num_reads as f64 / read_time.as_secs_f64(),
             read_avg,
             read_peak);

    // Export detailed memory CSV
    let mut csv_data = String::new();
    csv_data.push_str("phase,sample_num,memory_mb\n");

    for (i, &mem) in write_samples.iter().enumerate() {
        csv_data.push_str(&format!("write,{},{}\n", i, mem));
    }
    for (i, &mem) in read_samples.iter().enumerate() {
        csv_data.push_str(&format!("read,{},{}\n", i, mem));
    }

    let csv_file = bench_dir.join("memory_stress.csv");
    fs::write(&csv_file, &csv_data).ok();

    // Summary
    let summary = format!(
        "=== Memory Stress Test Summary ===\n\
         Write Phase:\n\
           Time: {:.2?}\n\
           Throughput: {:.0} ops/sec\n\
           Memory: avg={:.1} MB, peak={:.1} MB\n\
           SSTables created: {}\n\
         \n\
         Read Phase:\n\
           Time: {:.2?}\n\
           Throughput: {:.0} ops/sec\n\
           Memory: avg={:.1} MB, peak={:.1} MB\n\
         \n\
         CSV saved to: {}\n",
        write_time,
        num_keys as f64 / write_time.as_secs_f64(),
        write_avg,
        write_peak,
        db.num_of_ss_tables(),
        read_time,
        num_reads as f64 / read_time.as_secs_f64(),
        read_avg,
        read_peak,
        csv_file.display()
    );

    println!("{}", summary);

    let summary_file = bench_dir.join("memory_stress.txt");
    fs::write(&summary_file, summary).ok();

    db.purge();
}