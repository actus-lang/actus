# Phase 42 Region Performance Evidence — 2026-10-09

## Scope

This record measures the shared bounded bulk-copy path for hosted Region
operations at 64 KiB and 128 KiB. It is a Linux x86-64 host measurement and
does not establish embedded latency, memory, or power behavior.

## Reproduction

```sh
cargo bench --bench region
```

The benchmark uses the optimized Cargo bench profile. Scalar operation timing
uses 10,000 iterations. Bulk timing uses 100 iterations per payload size. The
benchmark allocates the resident window and caller buffers before timing each
bulk loop; allocation is not included in the copy measurement.

## Captured output

```text
REGION_OPS iterations=10000 avg_ns open=18 read=5 write=9 publish=13 cancel=13 remap=15 close=native_fixture
REGION_BULK iterations=100 bytes=65536 avg_ns read=949 write=936
REGION_BULK iterations=100 bytes=131072 avg_ns read=2079 write=1767
```

The measured operation is the runtime `read_range` or `write_range` path after
descriptor validation and exact buffer-size validation. Both operations use
the shared overlap-safe bulk-copy primitive. The 128 KiB case is the hosted
profile limit; lower target profiles can select a smaller limit through
`RegionAddressProfile::with_bulk_limit`.

These values are local observations. CPU frequency, compiler revision,
optimization settings, operating-system scheduling, and memory state can
change them. Compiler RSS, object size, executable size, and peak resident
memory were captured separately with a `/proc/<pid>/status` `VmHWM` poll while
running the strict compiler on resident-one-byte fixtures:

```text
REGION_PROFILE logical_bytes=65536 resident_bytes=1 status=0 build_ms=71 peak_rss_kb=22376 executable_bytes=5818568
REGION_PROFILE logical_bytes=131072 resident_bytes=1 status=0 build_ms=87 peak_rss_kb=22260 executable_bytes=5818568
REGION_ARTIFACT logical_bytes=65536 emit=obj status=0 build_ms=26 peak_rss_kb=22316 artifact_bytes=2120
REGION_ARTIFACT logical_bytes=65536 emit=exe status=0 build_ms=76 peak_rss_kb=22320 artifact_bytes=5818568
REGION_ARTIFACT logical_bytes=131072 emit=obj status=0 build_ms=31 peak_rss_kb=22308 artifact_bytes=2120
REGION_ARTIFACT logical_bytes=131072 emit=exe status=0 build_ms=92 peak_rss_kb=22028 artifact_bytes=5818568
```

The equal artifact sizes and near-constant compiler RSS show that these logical
extents do not materialize one native object per logical element. They remain
host measurements, not target or embedded guarantees.
