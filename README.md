# dbex

An LSM-tree based key-value storage engine built in Rust. Leverages Rust's zero-cost abstractions and memory safety to deliver high-throughput, low-latency storage with optimal I/O patterns and modern storage engine architecture.

## Features

- **Modern LSM-tree architecture**: MemTables (in-memory) + SSTables (on-disk)
- **Write-Ahead Log (WAL)**: LSN tracking each operation, actual recovery logic still TODO
- **Two-tier size based compaction**: Pre-compacted and compacted SSTable organization
- **Sparse indexing**: Fast lookups with range filtering
- **Memory-efficient**: 64MB MemTable flush threshold (may increase this)
- **Tombstone deletions**: Lazy deletion with compaction cleanup
- **Type-safe Rust implementation**: Using rkyv for fast serialization (could be optimized further)

## Architecture

### Storage Layers

**MemTable (L0 - In-Memory)**
- Active MemTable holds most recent operations as a BTreeMap
- Maintains sorted structure for efficient SSTable creation
- When reaching 64MB threshold, becomes immutable and a new MemTable is created
- Immutable MemTable is flushed to disk as an SSTable

**SSTables (On-Disk)**
- Each SSTable consists of a data file and an index file
- Index file maps keys to their offsets in the data file
- Sparse index: Every 100th key cached in memory for faster lookups

### Write Path

1. Write to active MemTable (in-memory)
2. When MemTable reaches 64MB, mark as immutable
3. Flush immutable MemTable to disk as new SSTable
4. Add to L0 (pre-compacted layer)
5. Create new active MemTable for incoming writes

### Compaction

**L0 → L1 Compaction** (triggered at 10 SSTables)
- K-way merge of all L0 SSTables into single L1 SSTable
- Removes duplicates (keeping newest values)
- Removes tombstones (deleted keys)

**L1 → L2 Compaction** (triggered at 10 SSTables)
- Same merge process as L0 → L1
- Further reduces read amplification

### Read Path

Keys are searched in order from newest to oldest:

```
MemTable → Immutable MemTable → L0 SSTables → L1 SSTables → L2 SSTables
 (RAM)   →       (RAM)         →    (Disk)   →    (Disk)   →    (Disk)
```

Range filtering using min/max keys allows skipping entire SSTables during lookups.

## Performance
Please see [BENCHMARKS](BENCHMARKS.md) for up-to-date performance tracking.

## Usage

```rust
// TODO
```

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Benchmarks

```bash
cargo test --release bench_
```

## Roadmap

### Planned Work
- Move from size-tiered compaction to level-tiered.
- Implement WAL recovery and truncating
- Further Compaction (reduce SSTable count and read amplification)
- Bloom filters (reduce disk seeks for missing keys)
- Block cache (cache hot SSTable blocks in RAM)
- Compression for both SS Tables and WAL (reduce disk I/O and storage)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
