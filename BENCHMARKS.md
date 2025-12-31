# Benchmark Results
This document outlines progress tracking of benchmarks from locals run on my M2 MacBook Pro.

### December 31st, 2025
#### Observations
Reads have become significantly faster after adding compaction, around a 2-3x speed increase.
Writes are now suffering since compaction is being done synchronously.

#### Results
```
============================================================
Benchmark: small
Keys: 100000, Value size: 100 bytes, Total data: 10.8 MB

============================================================

sequential_write         100000 ops in   370.92ms (      269596 ops/sec,     3.71 µs/op,     29.1 MB/s)
sequential_read           10000 ops in   118.32ms (       84513 ops/sec,    11.83 µs/op,      9.1 MB/s)
random_read               10000 ops in   131.68ms (       75944 ops/sec,    13.17 µs/op,      8.2 MB/s)
zipfian_read              10000 ops in   133.65ms (       74823 ops/sec,    13.36 µs/op,      8.1 MB/s)
L0 SSTables: 1
L1 SSTables: 0
L2 SSTables: 0

============================================================
Benchmark: medium
Keys: 1000000, Value size: 100 bytes, Total data: 108.0 MB

============================================================

sequential_write        1000000 ops in      4.03s (      248265 ops/sec,     4.03 µs/op,     26.8 MB/s)
sequential_read           10000 ops in   117.54ms (       85080 ops/sec,    11.75 µs/op,      9.2 MB/s)
random_read               10000 ops in   138.34ms (       72284 ops/sec,    13.83 µs/op,      7.8 MB/s)
zipfian_read              10000 ops in   140.95ms (       70948 ops/sec,    14.09 µs/op,      7.7 MB/s)
L0 SSTables: 2
L1 SSTables: 0
L2 SSTables: 0

============================================================
Benchmark: large
Keys: 1000000, Value size: 1000 bytes, Total data: 1008.0 MB

============================================================

sequential_write        1000000 ops in      6.70s (      149290 ops/sec,     6.70 µs/op,    150.5 MB/s)
sequential_read           10000 ops in     3.07ms (     3260382 ops/sec,     0.31 µs/op,   3286.5 MB/s)
random_read               10000 ops in    40.99ms (      243990 ops/sec,     4.10 µs/op,    245.9 MB/s)
zipfian_read              10000 ops in    17.77ms (      562650 ops/sec,     1.78 µs/op,    567.2 MB/s)
L0 SSTables: 5
L1 SSTables: 1
L2 SSTables: 0

============================================================
Benchmark: large_heavy_reads_and_writes
Keys: 1000000, Value size: 8000 bytes, Total data: 8008.0 MB

============================================================

sequential_write        1000000 ops in     18.26s (       54750 ops/sec,    18.26 µs/op,    438.4 MB/s)
sequential_read          100000 ops in    80.95ms (     1235273 ops/sec,     0.81 µs/op,   9892.1 MB/s)
random_read              100000 ops in   207.78ms (      481284 ops/sec,     2.08 µs/op,   3854.1 MB/s)
zipfian_read             100000 ops in   182.12ms (      549082 ops/sec,     1.82 µs/op,   4397.0 MB/s)
L0 SSTables: 10
L1 SSTables: 10
L2 SSTables: 0

============================================================
Benchmark: huge
Keys: 10000000, Value size: 1000 bytes, Total data: 10080.0 MB

============================================================

sequential_write       10000000 ops in    106.78s (       93651 ops/sec,    10.68 µs/op,     94.4 MB/s)
sequential_read           10000 ops in     4.87ms (     2051299 ops/sec,     0.49 µs/op,   2067.7 MB/s)
random_read               10000 ops in    14.32ms (      698092 ops/sec,     1.43 µs/op,    703.7 MB/s)
zipfian_read              10000 ops in    13.94ms (      717545 ops/sec,     1.39 µs/op,    723.3 MB/s)
L0 SSTables: 8
L1 SSTables: 2
L2 SSTables: 1
```

### December 17th, 2025
#### Observations
Writing this in retrospect, I can't remember what version this bench run was against. 
Most likely this was run using linear scanning of the index files, and not including a sparse index in memory.
It's also possible this was the addition of the sparse index in memory and binary search to find keys using that index.

You can see how not having compaction is killing performance when the number of sstables increases.

#### Results
```
============================================================
Benchmark: small
Keys: 100000, Value size: 100 bytes, Total data: 10.8 MB

============================================================
sequential_write         100000 ops in   375.99ms (      265967 ops/sec,     3.76 µs/op,     28.7 MB/s)
sequential_read           10000 ops in   100.95ms (       99057 ops/sec,    10.10 µs/op,     10.7 MB/s)
random_read               10000 ops in   113.71ms (       87947 ops/sec,    11.37 µs/op,      9.5 MB/s)
zipfian_read              10000 ops in   114.77ms (       87129 ops/sec,    11.48 µs/op,      9.4 MB/s)

Total SSTables: 1

============================================================
Benchmark: medium
Keys: 1000000, Value size: 100 bytes, Total data: 108.0 MB

============================================================
sequential_write        1000000 ops in      4.16s (      240630 ops/sec,     4.16 µs/op,     26.0 MB/s)
sequential_read           10000 ops in   101.85ms (       98179 ops/sec,    10.19 µs/op,     10.6 MB/s)
random_read               10000 ops in   119.54ms (       83652 ops/sec,    11.95 µs/op,      9.0 MB/s)
zipfian_read              10000 ops in   124.84ms (       80104 ops/sec,    12.48 µs/op,      8.7 MB/s)

Total SSTables: 2

============================================================
Benchmark: large
Keys: 1000000, Value size: 1000 bytes, Total data: 1008.0 MB

============================================================
sequential_write        1000000 ops in      4.96s (      201460 ops/sec,     4.96 µs/op,    203.1 MB/s)
sequential_read           10000 ops in   101.83ms (       98200 ops/sec,    10.18 µs/op,     99.0 MB/s)
random_read               10000 ops in   132.42ms (       75515 ops/sec,    13.24 µs/op,     76.1 MB/s)
zipfian_read              10000 ops in   131.91ms (       75808 ops/sec,    13.19 µs/op,     76.4 MB/s)

Total SSTables: 16

============================================================
Benchmark: large_heavy_reads_and_writes
Keys: 1000000, Value size: 8000 bytes, Total data: 8008.0 MB

============================================================
sequential_write        1000000 ops in     11.00s (       90920 ops/sec,    11.00 µs/op,    728.1 MB/s)
sequential_read          100000 ops in      1.60s (       62368 ops/sec,    16.03 µs/op,    499.4 MB/s)
random_read              100000 ops in      9.12s (       10960 ops/sec,    91.24 µs/op,     87.8 MB/s)
zipfian_read             100000 ops in      5.47s (       18265 ops/sec,    54.75 µs/op,    146.3 MB/s)

Total SSTables: 120

============================================================
Benchmark: huge
Keys: 10000000, Value size: 1000 bytes, Total data: 10080.0 MB

============================================================
sequential_write       10000000 ops in     47.57s (      210217 ops/sec,     4.76 µs/op,    211.9 MB/s)
sequential_read           10000 ops in   108.14ms (       92474 ops/sec,    10.81 µs/op,     93.2 MB/s)
random_read               10000 ops in      1.47s (        6798 ops/sec,   147.11 µs/op,      6.9 MB/s)
zipfian_read              10000 ops in      1.33s (        7499 ops/sec,   133.35 µs/op,      7.6 MB/s)

Total SSTables: 151
```
