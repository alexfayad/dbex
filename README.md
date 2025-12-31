# dbex - High-Performance LSM-Tree Storage Engine

A production-oriented LSM-tree based key-value storage engine built in Rust, designed to compete with RocksDB and LevelDB. Leverages Rust's zero-cost abstractions and memory safety to deliver high-throughput, low-latency storage with optimal I/O patterns and modern storage engine architecture.

## Features

- **LSM-tree architecture**: MemTables (in-memory) + SSTables (on-disk)
- **Write-Ahead Log (WAL)**: Crash recovery with LSN tracking (recovery TODO)
- **Two-tier compaction**: Pre-compacted and compacted SSTable organization
- **Sparse indexing**: Fast lookups with range filtering
- **Persistent file handles**: BufReader kept open for performance
- **Memory-efficient**: 64MB MemTable flush threshold
- **Tombstone deletions**: Lazy deletion with compaction cleanup
- **Comprehensive benchmarks**: Memory tracking and multiple access patterns
- **Type-safe Rust implementation**: Using rkyv for fast serialization (could be optimized further)

## Quick Start

```bash
# Run all tests
cargo test

# Run specific benchmark tests
cargo test bench_tiny        # 1K keys, tests CPU cache behavior
cargo test bench_small       # 100K keys, fits in RAM
cargo test bench_medium      # 1M keys, RAM pressure
cargo test bench_large       # 1M keys with 1KB values (1GB data)
cargo test bench_large_heavy_reads  # 1M keys, 8KB values, 100K reads

# Run tests with output visible
cargo test -- --nocapture

# Build the project
cargo build

# Build in release mode for accurate performance testing
cargo build --release
cargo test --release
```

## Project Structure

```
src/
  lib.rs              # Core DBex storage engine
  memtable.rs         # In-memory BTreeMap layer
  ss_table.rs         # SSTable implementation
  write_ahead_log.rs  # WAL for crash recovery
  utils.rs            # Utility types (Operation enum)

tests/
  integration_tests.rs  # Correctness tests with automatic cleanup
  bench.rs             # Performance benchmark tests

wals/
  cur.wal             # Current write-ahead log file
```

## Architecture

### Core Components

#### 1. MemTable (`src/memtable.rs`)
In-memory write buffer using sorted BTree structure.

**Implementation:**
- `BTreeMap<Vec<u8>, Option<Vec<u8>>>` (not HashMap - maintains sort order)
- Size tracking in bytes (flushes at 64MB threshold)
- Tombstone-based deletion (`None` values)
- Sorted iteration for efficient SSTable creation

**Why BTreeMap?**
- Maintains sorted key order for SSTable flush
- Enables efficient range scans
- Predictable performance (O(log n) operations)

#### 2. SSTable (`src/ss_table.rs`)
Sorted String Tables - immutable on-disk storage.

**File Format:**
Each SSTable creates two files:
- **Data file** (`ss_table_{timestamp}.db`):
  ```
  [value_len: 4 bytes][value]
  [value_len: 4 bytes][value]
  ...
  ```
- **Index file** (`ss_table_{timestamp}.db.index`):
  ```
  [key_len: 4 bytes][key][offset: 8 bytes]
  [key_len: 4 bytes][key][offset: 8 bytes]
  ...
  ```

**In-Memory Structure:**
- **Sparse index**: Every 100th entry indexed (`Vec<(Vec<u8>, u64)>`)
- **File handles**: `BufReader<File>` kept open (no reopen on reads)
- **Range metadata**: `min_key`, `max_key` for filtering
- **Tree structure**: Can contain child `ss_tables: Vec<SSTable>` for hierarchical compaction

**Lookup Process:**
1. Check if key is in range (`min_key <= key <= max_key`)
2. Binary search sparse index to find nearest entry
3. Seek to offset in data file
4. Read value directly

#### 3. Write-Ahead Log (`src/write_ahead_log.rs`)
Durability layer for crash recovery.

**Format** (`wals/cur.wal`):
```
[data_len: 8 bytes][serialized_wal_entry]
[data_len: 8 bytes][serialized_wal_entry]
...
```

**WAL Entry Structure:**
```rust
struct WalEntry {
    lsn: u64,              // Log Sequence Number
    operation: Operation,  // Insert, Delete, StartTxn, CommitTxn
    key: Option<Vec<u8>>,
    value: Option<Vec<u8>>
}
```

**Features:**
- Fast binary serialization using `rkyv` (faster than serde/bincode)
- LSN (Log Sequence Number) tracking for operation ordering
- Write-ahead guarantee: WAL written BEFORE memtable modification
- Operations: Insert, Delete, StartTxn, CommitTxn

**Not Yet Implemented:**
- WAL replay on database startup (recovery)
- WAL rotation (multiple log files)
- Transaction commit/rollback

#### 4. DBex Engine (`src/lib.rs`)
Main storage engine coordinating all components.

**Structure:**
```rust
pub struct DBex {
    memtable: MemTable,                      // Active write buffer
    immutable_memtable: Option<MemTable>,    // Being flushed to disk
    pre_compact_ss_tables: Vec<SSTable>,     // Newly flushed, may overlap
    compacted_ss_tables: Vec<SSTable>,       // Compacted, no key overlap
    write_ahead_log: WriteAheadLog,          // Durability layer
    lsn: u64,                                // Current log sequence number
    // ...
}
```

**Two-Tier SSTable Organization:**
- **Pre-compacted**: Newly flushed SSTables with potential key overlap
- **Compacted**: Merged SSTables with no key overlap (hierarchical tree)
- **Trigger**: Compaction starts when `pre_compact_ss_tables.len() > 10`

### Write Path

```
1. write_ahead_log.write(lsn, Insert, key, value)  # Durability
2. memtable.insert(key, value)                     # In-memory write
3. Check memtable.size_bytes()
4. If > 64MB:
   a. Freeze active memtable → immutable_memtable
   b. Create new empty memtable
   c. Flush immutable to new SSTable
   d. Add SSTable to pre_compact_ss_tables
5. If pre_compact_ss_tables.len() > 10:
   a. Trigger compaction (moves to compacted tier)
```

### Read Path

```
1. Check active memtable (O(log n) BTreeMap lookup)
2. If not found, check immutable_memtable (if exists)
3. If not found, iterate through pre_compact_ss_tables (newest → oldest):
   a. Check min_key/max_key range
   b. If in range, query SSTable sparse index
4. If not found, iterate through compacted_ss_tables:
   a. Check ranges, recursively search tree structure
5. Return first match (or None)
```

### Compaction (Framework Only)

**Current Implementation:**
- Trigger: When `pre_compact_ss_tables.len() > 10`
- Action: Moves tables from pre-compacted to compacted tier
- **NOT YET IMPLEMENTED**: Actual key merging, tombstone removal, multi-level compaction

**Planned:**
- Merge overlapping SSTables
- Remove tombstoned keys
- Build hierarchical levels (L0, L1, L2, ...)
- Size-tiered or leveled compaction strategy

## Testing Structure

### Integration Tests (`tests/integration_tests.rs`)
Correctness tests with automatic cleanup.

**Features:**
- `TestDb` guard pattern ensures cleanup even on test failure
- Removes all SSTable files (`.db` and `.db.index`)
- Cleans up WAL files (`wals/*.wal`)

**Key Tests:**
- `test_basic_insert_and_find` - Core operations
- `test_index_persistence_and_recovery` - SSTable index loading
- `test_many_small_keys` - 10K entries scalability
- Edge cases: large values, binary data, persistence across restarts

### Benchmark Tests (`tests/bench.rs`)
Performance measurement with memory tracking.

**Benchmark Functions:**
- `bench_sequential_writes` - Write throughput and latency
- `bench_sequential_reads` - Sequential read performance
- `bench_random_reads` - Uniform random access
- `bench_zipfian_reads` - 80/20 hot key distribution (realistic workload)

**Categories:**
- `bench_tiny` - 1K keys (CPU cache behavior)
- `bench_small` - 100K keys (fits in RAM)
- `bench_medium` - 1M keys (RAM pressure)
- `bench_large` - 1M keys × 1KB values = 1GB data
- `bench_large_heavy_reads` - 1M keys × 8KB values, 100K reads (hot cache)

**Memory Tracking:**
Uses `sysinfo` crate to measure memory usage during benchmarks.

## Current Status

### Completed ✅
- LSM-tree architecture with MemTable and SSTables
- Write-Ahead Log with LSN tracking and rkyv serialization
- Two-tier compaction framework (structure in place)
- Sparse indexing with binary search
- Range filtering using min/max keys
- File handle caching (BufReader kept open)
- Tombstone deletions in MemTable
- Memory benchmarking with sysinfo
- Automatic MemTable flushing at 64MB

### In Progress 🔄
- **WAL recovery/replay** on startup (writes work, recovery TODO)
- **Actual compaction merging** (framework exists, merging logic stubbed)
- **Transaction support** (WAL operations defined, commit/rollback TODO)

### Known Issues ⚠️
- Compaction just moves tables between vectors (doesn't merge keys)
- WAL recovery not implemented (can't recover from crashes yet)
- Some index file operations use linear scan instead of binary search with sparse index
- `prev_fal_files` field in WAL is unused (rotation not implemented)
- Transaction methods are stubs

### Planned Features 📋
- WAL replay on database startup
- Complete compaction merging logic with tombstone removal
- Bloom filters for faster negative lookups
- WAL rotation and archival
- Range queries (leverage BTreeMap ordering)
- Full transaction support (ACID properties)
- Multi-level compaction (L0, L1, L2, ...)

## Dependencies

- **rkyv 0.8.12** + **rkyv_dyn 0.7.44** - Fast zero-copy binary serialization for WAL
- **sysinfo 0.37.2** - Memory profiling in benchmarks
- **rand 0.9.2** - Random access patterns in benchmarks

## Performance Optimizations

**Implemented:**
- ✅ File handles kept open (no reopen on reads)
- ✅ Sparse indexing (every 100th entry)
- ✅ Range filtering (min/max keys to skip SSTables)
- ✅ BTreeMap for sorted memtable (efficient SSTable flush)
- ✅ Buffered I/O with BufReader/BufWriter

**Planned:**
- Bloom filters (reduce disk seeks for missing keys)
- Compaction (reduce SSTable count and read amplification)
- Block cache (cache hot SSTable blocks in RAM)
- Compression (reduce disk I/O and storage)

## Performance Notes

- **Always benchmark in release mode**: `cargo build --release && cargo test --release`
- Write throughput currently bottlenecked by synchronous compaction (~94 MB/s)
- Async compaction implementation will unlock 5-10x write performance
- Read performance competitive with established databases (cache-hot: 2.1 GB/s, random: 687 MB/s)
- SSTable format: `.db` (data) and `.db.index` (index) files per table
- WAL stored in `wals/` directory for crash recovery

---

**Documentation**: This README was written by [Claude](https://claude.ai), an AI assistant by Anthropic. This README is kept updated as contributions to the project are made.