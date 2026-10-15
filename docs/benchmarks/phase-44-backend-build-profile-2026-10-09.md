# Phase 44 Backend Build Profile — 2026-10-09

## Scope

This record compares the opt-in Cranelift-only and LLVM-only compiler builds
on one Linux x86-64 development host. It records installation requirements,
direct Cargo dependency surfaces, clean debug build time, target-directory
size, and debug executable size. These are host observations; they are not
release-size, cross-platform, or embedded measurements.

## Toolchain and host

- Actus revision: `15209ac7ee75c4b2484debd484c03cc09890e31a`
- Host: Linux x86-64
- Rust: `rustc 1.98.0 (88d9e12ae 2026-08-18)`
- LLVM: `llvm-config 23.1.1`
- Cargo profile: `dev`
- Build parallelism: `CARGO_BUILD_JOBS=1`
- Target directories: new empty directories under `/tmp`

## Installation requirements

The default feature set is Cranelift and does not require LLVM, LLVM headers,
LLVM libraries, or `llvm-sys`. The LLVM configuration requires LLVM 23.1.x,
`llvm-config` discovery, and the `LLVM_SYS_231_PREFIX` environment variable
when discovery is not available through the default executable path.

The repository manifest keeps `llvm-sys` optional and enables its strict
version policy. LLVM is therefore an installation prerequisite only for the
LLVM feature configuration; it is not a prerequisite for Cranelift-only use.

## Reproduction commands

```sh
cranelift_dir=/tmp/actus-measure-cranelift-20261009
llvm_dir=/tmp/actus-measure-llvm-20261009

CARGO_BUILD_JOBS=1 \
  CARGO_TARGET_DIR="$cranelift_dir" \
  cargo build --no-default-features --features cranelift --lib --bin actus

LLVM_SYS_231_PREFIX=/usr \
  CARGO_BUILD_JOBS=1 \
  CARGO_TARGET_DIR="$llvm_dir" \
  cargo build --no-default-features --features llvm --lib --bin actus
```

The two builds use the same package, Rust toolchain, profile, host, and clean
target-directory condition. Cargo's dependency download and registry cache
are outside the measured target-directory sizes.

## Direct dependency surface

`cargo tree --depth 1` reports these backend-specific direct dependencies:

| Configuration | Backend-specific direct dependencies |
| --- | --- |
| Cranelift-only | `cranelift-codegen 0.136.2`, `cranelift-frontend 0.136.2`, `cranelift-module 0.136.2`, `cranelift-object 0.136.2` |
| LLVM-only | `llvm-sys 231.0.1` |

Both configurations retain the shared compiler dependencies. `llvm-sys` is
absent from the Cranelift-only dependency tree, and the Cranelift crates are
absent from the LLVM-only dependency tree.

## Captured host measurements

| Configuration | Clean build time | Debug executable | Target directory |
| --- | ---: | ---: | ---: |
| Cranelift-only | 45 s | 139,775,272 bytes | 1,101,882,753 bytes |
| LLVM-only | 20 s | 58,899,008 bytes | 574,551,633 bytes |

The target-directory and executable values include unoptimized debug artifacts
and are useful for comparing this exact build setup only. They do not predict
release binaries, CI cache size, installation package size, or runtime memory.
The build times include compilation of dependencies in empty target
directories and exclude registry download time.

## CI coverage

The workflow runs the two configurations independently:

- `cranelift-only` runs `cargo check` and the complete all-target test suite
  without LLVM.
- `llvm-only` checks the library and CLI compiler surface and runs
  `tests/codegen/llvm/backend.rs` without Cranelift.

The LLVM-only job intentionally does not run Cranelift-specific native test
modules. Those tests reference the Cranelift backend API and are outside the
LLVM-only feature contract until shared cross-backend fixtures exist.

## Evidence boundary

This profile proves dependency and build isolation on the recorded host. It
does not prove ARM32/Thumb code generation, external C ABI compatibility,
freestanding linking, target execution, or production performance. Those
claims remain open under the later Phase 44 gates.
