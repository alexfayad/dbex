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

In-memory table (MemTable) holds most recent operations until it reaches a certain size. This is stored as a BTreeMap which maintains a sorted structure required by our application when flushing to an SSTable.

When the MemTable reaches 64MB in size, we clone the _active_ MemTable and mark it as immutable, then create a new MemTable to keep handling incoming writes.

The immutable table then gets flushed to disk as a Sorted Strings Table (SS Table), which is fairly simple since it's already sorted.

We maintain an index file alongside the SS Table's data file, which serves as our lookup for where the data lives for each key.

When creating this index file, we also keep in memory a reference to every 100th key, as a sparse index, allowing us to find a given key much more rapidly.

Once the SS Table is created, we add it to the list of L0 SS Tables, these tables are pre-compacted, within a given level, the same key can exist multiple times.

Once L0 reaches a length of 10, this triggers compaction, the SS Tables are then compacted into one larger table, removing all duplicates and deleted values.

This new larger table gets added to the L1 SS Tables, this process is then repeated for L2 SS Tables.

When fetching a key, we look for the value in the following order:

MemTable -> Immutable MemTable -> L0 SS Tables -> L1 SS Tables -> L2 SS Tables
The boundaries are as follows:
Memory -> Memory -(slower after this)-> Disk -> Disk -> Disk

## Performance
Please see [BENCHMARKS](BENCHMARKS.md) for up to date performance tracking.

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
