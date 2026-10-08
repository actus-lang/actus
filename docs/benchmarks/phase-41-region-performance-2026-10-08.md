# Phase 41 Region Performance Evidence — 2026-10-08

## Scope

This record covers the bounded bulk limit, hosted operation costs, and logical
capacity behavior of `std::region`. It measures the runtime-owned in-memory
boundary; it does not claim embedded hardware latency.

## Bulk contract

- Maximum payload for one `region_read_range` or `region_write_range` call:
  64 KiB.
- A larger checked payload returns `BulkLimitExceeded` before copying.
- Core provider queue capacity: zero. The core performs no implicit queued
  asynchronous work.
- Larger transfers must be chunked explicitly by the caller or by a provider
  adapter with its own bounded queue contract.

## Reproduction

```sh
cargo bench --bench region
cargo test --test std_region_scale -- --nocapture --test-threads=1
```

Compiler source revision for the operation benchmark: `6e605af`, with the
Gate 41.11 working tree changes applied. Host profile: Linux x86-64, optimized
Rust benchmark profile, hosted runtime. `ITERATIONS=10000`.

## Hosted operation costs

The benchmark reports average elapsed nanoseconds per operation over 10,000
iterations on the local host:

```text
REGION_OPS iterations=10000 avg_ns open=20 read=6 write=9 publish=13 cancel=12 remap=15 close=native_fixture
```

`close` is covered by the hosted native lifecycle fixture because it operates
through the capability table and C ABI bridge; it is not represented as a
direct `InMemoryRegion` method benchmark. The fixture is
`benchmarks/region_close/src/main.act` and runs 10,000 iterations of the
complete native lifecycle. On the same host it reported:

```text
explicit-close-lifecycle-ns=6686162
lexical-drop-baseline-ns=3150159
explicit-close-delta-ns=3536003
explicit-close-delta-per-iteration-ns=353
```

The delta is a controlled comparison between the same open and cleanup path
with and without an explicit `region_close`; it is a close-path estimate, not
a hardware latency guarantee.

These values are local observations, not latency guarantees. CPU frequency,
compiler version, optimization profile, OS scheduling, and allocation state
can change them.

## Logical capacity and resident storage

The scale fixture uses one resident `u8` byte or one resident 64-byte packed
element while increasing logical length from 1 MiB through 1 TiB:

```text
u8         resident=1  executable=5810296  status=0
Minicolumn resident=64 executable=5810312  status=0
```

All six scale cases completed successfully. The executable size remained
constant for each element type across 1 MiB, 1 GiB, and 1 TiB logical lengths,
which is evidence that logical extent does not materialize one native object
per logical position.

## Allocation and I/O boundary

Read, write, publish, cancel, inspection, pin, and unpin paths operate on
existing resident storage and perform no filesystem, device, provider, or
implicit page-fault I/O. Opening and remapping intentionally acquire bounded
resident storage; the benchmark's remap measurement includes that explicit
replacement allocation. Close operates on the bounded capability table and
does not perform external I/O. The provider boundary has separate explicit
load/store/flush/cancel/recover operations; no asynchronous queue is hidden in
the Region core.

Compiler RSS, object size, and symbol tables remain covered by the Phase 38
scale evidence. A current revision snapshot was also captured from the native
close fixture with strict compilation and zero-float verification:

```text
object_status=0 compiler_peak_rss_kb=25288 object_bytes=7600 defined_symbols=3
executable_status=0 compiler_peak_rss_kb=30704 executable_bytes=5831768
```

The RSS values are peak child-process values from the hosted Linux x86-64
measurement process. Artifact sizes and symbol count describe this fixture;
the logical-capacity scale result above remains the evidence for bounded
specialization across 1 MiB, 1 GiB, and 1 TiB logical lengths.
