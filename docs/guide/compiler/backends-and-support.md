# Backends and support matrix

Actus has two native emitters. LLVM is the default native backend; Cranelift
remains an explicit compatibility backend. The emitter is selected after
parsing, semantic analysis, ownership checking, generic specialization, and
shared lowering. Backend selection therefore changes machine emission, not
Actus source semantics, ownership roles, cleanup rules, or serialized data.

Both backends consume the same checked semantic and lowering contract. A
backend change is therefore not a change to the language's ownership model or
to the meaning of an Actus program; it changes only the final native emission
and target-linking path.

## Installation and availability

The compiler feature set controls which emitters are available:

| Backend | Cargo feature | Version in this repository | External requirement |
| --- | --- | --- | --- |
| LLVM | `llvm` (default) | `llvm-sys 231.0.0` | LLVM 23.x installation and matching `LLVM_SYS_231_PREFIX` |
| Cranelift | `cranelift` (explicit) | `0.137.0` | Rust/Cargo only |

The default build enables LLVM and therefore requires the matching LLVM
toolchain:

```sh
cargo build --bin actus
```

For a compiler installation that deliberately avoids LLVM, build the explicit
Cranelift-only variant:

```sh
cargo build --no-default-features --features cranelift --bin actus
```

An installation with both emitters enabled is the parity-validation and
development configuration. LLVM remains the default in that configuration;
Cranelift is selected only by an explicit manifest or command-line override:

```sh
cargo build --all-features --bin actus
```

LLVM availability is checked when the `llvm` feature is compiled. The LLVM
installation must provide the version selected by `llvm-sys`, and the native
linker and target libraries must be available for the requested target.
Cranelift does not require a system LLVM installation. A backend that was not
compiled into the `actus` binary is rejected with a typed availability error;
the compiler never silently switches to the other backend. Enabling both
features does not create an implicit fallback; it only makes both backends
available for explicit selection.

## Selecting a backend

The selection order is explicit and deterministic:

1. a valid `--backend` command-line option overrides the project default for
   that invocation;
2. otherwise, `build.backend` in `Actus.toml` selects the project backend;
3. when neither is present, the compiler uses LLVM, the default enabled
   backend;
4. in a Cranelift-only compiler build, Cranelift becomes the available default
   because LLVM is not compiled into that binary.

An explicit selection is validated against the compiler feature set and the
target contract. An unavailable selection is an error, not a request to try a
different backend.

The backend may be selected in `Actus.toml`:

```toml
[build]
backend = "llvm"
target = "x86_64-unknown-linux-gnu"
runtime = "std"
```

To pin the compatibility backend for the same project, use
`backend = "cranelift"`. This changes only native emission; it does not alter
Actus source syntax, ownership roles, cleanup behavior, or serialized data.

It may also be overridden for one invocation:

```sh
actus build --backend cranelift --emit exe -o build/app
actus build --backend llvm --emit exe -o build/app
```

The selected backend is recorded in native artifact metadata. Changing it
invalidates stale generated artifacts rather than reusing an object emitted by
the other backend. The sidecar metadata records compiler and toolchain
identity, backend, target triple and target-contract hash, profile, runtime,
source-limit policy, and runtime version/checksum when applicable. The build
graph compares these fields before reusing an artifact. A backend, target,
runtime, profile, or incompatible toolchain change therefore invalidates the
generated artifact family and causes a rebuild.

Use `actus build --emit obj` when inspecting object symbols or freestanding
imports. Use `actus build --emit exe` only when the target's entry, runtime,
and linker contract is complete. An object emitted by one backend is not
evidence that the other backend can link or execute the same target.

## Supported target profiles

Target support is a contract, not a promise that every source feature works
on every triple. The compiler validates pointer width, endianness, ABI, entry
contract, linker flavor, runtime provider, and backend availability before
native emission.

| Profile | Representative targets | Entry | Runtime | Evidence |
| --- | --- | --- | --- | --- |
| Hosted Linux | `x86_64-unknown-linux-gnu` | host runtime and `main` | `std` hosted provider | hosted CLI and library suites |
| Hosted Apple | `x86_64-apple-darwin` | host runtime | configured hosted provider | target/linker contract tests |
| Hosted Windows | `x86_64-pc-windows-msvc` | host runtime | configured hosted provider | target/linker contract tests |
| Freestanding UEFI/object | `x86_64-unknown-uefi` | explicit entry | explicit provider | object and Region/Wire parity suites |
| Freestanding Cortex-M | `thumbv7em-none-eabihf` | explicit entry, Thumb-2 | explicit provider and linker script | LLVM ARM object/link tests and target policy tests |

The hosted rows are executable-link targets when the required system linker and
runtime libraries are installed. Freestanding rows are object/link-contract
targets unless a complete startup, linker script, runtime provider, and target
toolchain are supplied. Passing a host test does not establish execution on a
microcontroller.

Both backends consume the same shared lowering contract for the supported
parity slice. The current evidence includes hosted executable parity,
freestanding object parity, target-aware float policy, aggregate layout and
cleanup, runtime bridges, and negative diagnostics. Unsupported source shapes
must fail at the shared semantic/native boundary with a typed diagnostic.

## Runtime providers and object formats

`runtime = "std"` resolves hosted standard-library providers only where the
target contract supplies those services. A freestanding target must provide
the required runtime bridges explicitly; it must not inherit filesystem,
console, clock, allocation, or hosted startup services implicitly.

The target determines the object format and linker flavor:

| Target family | Object format | Linker contract |
| --- | --- | --- |
| GNU/Linux and freestanding ELF | ELF | GNU-style `cc`/linker arguments |
| Apple targets | Mach-O | Apple `cc`/linker arguments |
| MSVC targets | COFF | `lld-link.exe` and `.lib` arguments |

For freestanding Cortex-M, the target contract requires an explicit entry
symbol, a linker script, an explicit provider, and the declared stack
alignment. For hosted executables, the configured linker supplies the host
startup contract. `ACTUS_LINKER` may override the linker executable without
changing source semantics.

## Troubleshooting

| Symptom | Classification | Correct action |
| --- | --- | --- |
| `backend llvm is not available` | compiler feature availability | rebuild with `--features llvm`; do not expect Cranelift fallback |
| LLVM headers/library not found | LLVM installation | install matching LLVM 23.x and set `LLVM_SYS_231_PREFIX` |
| linker executable not found | external linker | install/configure the target linker or set `ACTUS_LINKER` |
| missing sysroot, startup, or linker script | target contract | provide the target sysroot/startup/provider; an object build alone is not an executable proof |
| missing runtime bridge | runtime provider/linkage | enable the correct runtime profile or add the explicit target provider |
| target/backend incompatibility | target validation | select a supported target/backend pair; do not wrap or rewrite the source |
| large aggregate rejected on freestanding | intentional target policy | keep the aggregate below `LARGE_AGGREGATE_THRESHOLD` or provide explicit target storage; implicit heap allocation is forbidden |
| unsupported operation diagnostic | shared lowering boundary | preserve the complete span/backend/target diagnostic; do not switch backends silently |
| stale object after changing backend/target | build graph invalidation | rebuild; artifact metadata intentionally invalidates the old object |

When diagnosing a failure, retain the full command and diagnostic, then
classify it as semantic, module visibility, shared lowering, backend emission,
runtime linkage, target configuration, or external linker failure. A linker
error must not be used to hide a missing shared contract.

## Clean checkout and backend verification

Use a clean checkout when validating an installation or investigating a
backend-selection problem. This separates toolchain availability from stale
objects left in an older target directory:

```sh
git clone <repository-url> actus-clean
cd actus-clean
git checkout <revision>
cargo build --bin actus
./target/debug/actus check examples/hello.act --strict
./target/debug/actus test --strict
```

The default build in this checkout is LLVM. Before the first Cargo command,
install LLVM 23.x for the host and expose the matching installation through
`LLVM_SYS_231_PREFIX` when `llvm-config` is not already discoverable. The
variable only tells `llvm-sys` where LLVM is installed; it does not install
LLVM, the native linker, system libraries, or a target SDK.

To validate the compatibility path from the same clean checkout, use a
separate target directory and disable default features:

```sh
CARGO_TARGET_DIR=target-cranelift \
  cargo build --no-default-features --features cranelift --bin actus
CARGO_TARGET_DIR=target-cranelift \
  cargo test --no-default-features --features cranelift --all-targets -- --test-threads=1
```

Do not copy objects between these target directories. The compiler records the
backend in artifact metadata, while separate directories also make the chosen
toolchain visible in local and CI logs.

## Rollback and compatibility

The migration does not change Actus source syntax, ownership roles,
serialized runtime layouts, or public standard-library contracts. To roll back
the native emitter for a project, set `backend = "cranelift"` in
`Actus.toml`, or pass `--backend cranelift` for one build/run. The compiler
must have been built with the `cranelift` feature. Rebuild the artifact so its
metadata matches the selected backend; no source rewrite or serialized-data
migration is required. For a toolchain-level rollback, build Actus with
`--no-default-features --features cranelift`. This is an explicit rollback,
not a silent fallback.

This document records supported evidence, not a claim that every Actus program
or every target triple is complete. New target or runtime support requires its
own target contract, ADR/roadmap evidence, and parity tests.
