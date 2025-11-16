# dbex - A High-Performance Document Database

A document database built in Rust, designed for performance and learning database internals.

## Features

- Document-based storage (JSON objects)
- CRUD operations (Create, Read, Update, Delete)
- File persistence
- Benchmarking framework with MongoDB comparison
- Type-safe Rust implementation

## Quick Start

```bash
# Run tests
cargo run

# Run benchmarks
cargo bench

# Run MongoDB comparison benchmarks
cargo bench --bench comparison_bench
```

## Project Structure

```
src/
  lib.rs      # Core database engine
  main.rs     # Example usage and tests
  bin/
    mongo_bench.rs  # MongoDB benchmark tool

benches/
  insert_bench.rs      # Insert performance benchmarks
  query_bench.rs       # Query performance benchmarks
  throughput_bench.rs   # Throughput benchmarks
  comparison_bench.rs   # MongoDB comparison benchmarks

scripts/
  setup_mongo.sh       # MongoDB setup script
  compare_with_mongo.sh # Comparison script
```

## Current Status

**Phase 1: Foundation** ✅
- Basic storage (HashMap in memory)
- File persistence (JSON)
- CRUD operations
- Benchmarking framework

**Next Steps:**
- Indexing (B-Tree indexes)
- Write batching
- Better serialization format
- Query optimization

## Benchmarks

View benchmark results in `target/criterion/` after running `cargo bench`.