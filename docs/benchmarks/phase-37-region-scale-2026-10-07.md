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

## Captured output

```text
REGION_SCALE logical_label=1MiB logical_bytes=1048576 resident_bytes=1 build_ms=64 executable_bytes=5775568 run_us=714 status=0
REGION_SCALE logical_label=1GiB logical_bytes=1073741824 resident_bytes=1 build_ms=64 executable_bytes=5775568 run_us=645 status=0
REGION_SCALE logical_label=1TiB logical_bytes=1099511627776 resident_bytes=1 build_ms=64 executable_bytes=5775568 run_us=671 status=0
```

The measurements show constant resident capacity and executable size across
the three logical extents. Build and run timings are one host observation, not
a portable performance claim.

## Evidence boundary

- Logical capacity: 1 MiB, 1 GiB, and 1 TiB of one-byte elements.
- Resident capacity: one byte in every case.
- Storage mode: bounded in-memory resident window through the hosted Region
  bridge; no filesystem or mapped-file backend is involved.
- Native output: strict executable with the compiler's zero-float verification.
- Not measured here: peak compiler RSS, release optimization behavior,
  freestanding output, embedded hardware latency, and persistent storage.
