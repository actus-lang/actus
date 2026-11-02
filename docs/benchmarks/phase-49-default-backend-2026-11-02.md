# Default backend migration measurements — 2026-11-02

## Scope and evidence boundary

This record measures the default LLVM compiler and the explicit Cranelift-only
compiler on one Linux x86-64 development host. It covers clean compiler build
time, warm Cargo rebuild time, compiler and target-directory size, native
example build time, and native execution. These are local host measurements;
they are not release-size, cross-platform, embedded, or production-throughput
claims.

## Reproduction environment

- Actus revision: `3224d1c0e01446957c36e7006c4aae5916ab310f`
- Host: x86-64 Linux
- Rust: `rustc 1.98.0 (88d9e12ae 2026-08-18)`
- LLVM: `llvm-config 23.1.3`
- Cargo profile: `dev`
- Build parallelism: `CARGO_BUILD_JOBS=1`
- Target directories: separate new directories under `/tmp`
- Source fixture: `examples/hello.act`
- Expected native result: exit code `42`

The LLVM compiler was built with the repository default feature set. The
Cranelift compiler was built independently with `--no-default-features
--features cranelift`. This prevents one backend's dependencies or generated
objects from being counted as the other backend's clean build.

## Commands

```sh
LLVM_TARGET=/tmp/actus-49-measure-llvm
CRANELIFT_TARGET=/tmp/actus-49-measure-cranelift

CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR="$LLVM_TARGET" \
  cargo build --bin actus
CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR="$LLVM_TARGET" \
  cargo build --bin actus

CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR="$CRANELIFT_TARGET" \
  cargo build --no-default-features --features cranelift --bin actus
CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR="$CRANELIFT_TARGET" \
  cargo build --no-default-features --features cranelift --bin actus

"$LLVM_TARGET/debug/actus" build examples/hello.act --strict \
  --emit exe -o /tmp/hello-llvm
/tmp/hello-llvm

"$CRANELIFT_TARGET/debug/actus" build examples/hello.act --strict \
  --emit exe -o /tmp/hello-cranelift
/tmp/hello-cranelift
```

The two consecutive compiler builds with the same target directory measure
clean compilation and a no-source-change Cargo cache hit. Native build and run
times were measured separately with shell timestamps. `hello.act` returns 42
by design; that result is the success condition for both executables.

## Results

| Measurement | LLVM default | Cranelift-only |
| --- | ---: | ---: |
| Clean compiler build | 22,909 ms | 47,952 ms |
| Warm compiler rebuild | 57 ms | 38 ms |
| Compiler executable | 101,954,824 bytes | 177,938,504 bytes |
| Cargo target directory | 864,763,890 bytes | 1,380,849,377 bytes |
| `hello.act` native build | 41 ms | 37 ms |
| `hello.act` native run | 2 ms | 2 ms |
| Native exit status | 42 | 42 |

The warm rebuilds completed without recompiling source dependencies, showing
that the backend-specific target directories retain independent Cargo cache
state. The LLVM compiler and target directory were smaller in this exact
debug configuration, while the native example build and execution times were
effectively equivalent at the measurement resolution.

## Interpretation and limits

These measurements support the default selection decision for the recorded
host: LLVM has a lower clean compiler build time and smaller debug compiler
and target-directory artifacts in this checkout, while both emitters build
and execute the same integer-only example with the same result. They do not
establish release performance, runtime throughput, memory usage, linker
performance on other platforms, or embedded target behavior. A later release
benchmark must repeat the measurement with pinned release profiles and the
relevant target toolchains.
