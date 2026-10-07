# Phase 37 Region Scale Evidence — 2026-10-07

## Scope

This is hosted Linux x86-64 evidence for the runtime-backed `Region[u8]`
representation. Each case uses a one-byte resident window and changes only
the logical length. The test does not allocate the logical extent and does not
claim embedded or freestanding behavior.

## Reproduction

From the Actus repository root:

```text
cargo test --test std_region_scale -- --nocapture
```

The test invokes the strict Actus compiler executable and runs each generated
native executable. The compiler profile is the Cargo test profile. The host is
the configured Linux x86-64 development machine. Compiler source revision:
`8e36e73` (`test: reject invalid region states natively`).

Peak compiler RSS was measured separately by polling the child compiler's
`/proc/<pid>/status` `VmHWM` field while running the same strict build shape.
This is a Linux host-process measurement and is not an embedded memory bound.

## Captured output

```text
REGION_SCALE logical_label=1MiB logical_bytes=1048576 resident_bytes=1 build_ms=64 executable_bytes=5775568 run_us=714 status=0
REGION_SCALE logical_label=1GiB logical_bytes=1073741824 resident_bytes=1 build_ms=64 executable_bytes=5775568 run_us=645 status=0
REGION_SCALE logical_label=1TiB logical_bytes=1099511627776 resident_bytes=1 build_ms=64 executable_bytes=5775568 run_us=671 status=0
```

The corresponding compiler-process measurements were:

```text
RSS_EVIDENCE label=1MiB peak_rss_kb=21760 build_ms=62 executable_bytes=5775568 exit=0
RSS_EVIDENCE label=1GiB peak_rss_kb=21608 build_ms=69 executable_bytes=5775568 exit=0
RSS_EVIDENCE label=1TiB peak_rss_kb=21640 build_ms=77 executable_bytes=5775568 exit=0
```

The measurements show constant resident capacity, peak compiler RSS, and
executable size across the three logical extents. Build and run timings are
one host observation, not a portable performance claim.

## Evidence boundary

- Logical capacity: 1 MiB, 1 GiB, and 1 TiB of one-byte elements.
- Resident capacity: one byte in every case.
- Storage mode: bounded in-memory resident window through the hosted Region
  bridge; no filesystem or mapped-file backend is involved.
- Native output: strict executable with the compiler's zero-float verification.
- Not measured here: release optimization behavior, freestanding output,
  embedded hardware latency, and persistent storage.

## Inline aggregate comparison

The current largest checked inline fixture is
`tests/fixtures/scalable_aggregate/aggregate_1048576.act`. It was validated
and built with:

```text
target/debug/actus check tests/fixtures/scalable_aggregate/aggregate_1048576.act --strict
target/debug/actus build tests/fixtures/scalable_aggregate/aggregate_1048576.act --strict --emit exe -o /tmp/actus-scale-1048576.aie
INLINE_SCALE executable_bytes=23830616
```

This fixture is a real inline aggregate and therefore has materially different
storage behavior from the Region cases above. Its successful build is recorded
as the current hosted practical inline extent; it must not be interpreted as a
claim that arbitrary larger inline aggregates are safe or memory-bounded.
