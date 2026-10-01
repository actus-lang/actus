# ADR-0050: Runtime Profiles and Builtin Standard-Library Resolution

- Status: Proposed
- Date: 2026-10-01
- Scope: Actus package runtime selection and compiler-owned standard-library resolution

## Context

Actus packages currently describe standard-library modules as ordinary local
dependencies. A new project therefore has to copy or point at `library/std`
manually, and an absolute host path can accidentally become part of the
project's build contract. That makes a first build dependent on the checkout
layout rather than on the compiler that owns the runtime.

The standard library must also work for more than hosted desktop programs.
Embedded targets such as `thumbv7em-none-eabihf` have no operating system, but
they still need compiler-owned libraries for buffers, embedded protocols,
register access, and peripheral abstractions. A single hosted-only `std`
definition would make the library unusable exactly where Actus's bounded and
deterministic model is most valuable.

The compiler already has a manifest-driven configuration boundary. Runtime
selection belongs there, next to target and profile selection, while module
resolution must remain responsible for locating and loading reachable source
files. The two concerns must not be mixed: a runtime profile selects a
compiler-owned environment; it does not make every runtime module an implicit
dependency.

## Decision

Actus packages may select one runtime profile in `[build]`:

```toml
[build]
runtime = "std"
```

The accepted values are:

- `core`: dependency-free language and compiler core. This is the default when
  `runtime` is omitted.
- `std`: the compiler-owned, target-aware standard library, resolved from a
  versioned builtin/sysroot location. Applications may use canonical
  `std::...` imports without copying library sources into the package. The
  target contract decides which modules are available; hosted-only modules
  such as filesystem access are not exposed on a freestanding target, while
  target-neutral and embedded modules remain available.
- `freestanding`: no hosted runtime is acquired implicitly. The target must
  provide the entry and platform services explicitly. This profile is for
  packages that intentionally want only core and explicit target libraries;
  it does not prevent a freestanding project from selecting the target-aware
  `std` profile when standard-library modules are desired.

Runtime profile is a typed configuration value, not a package dependency name.
The resolver must keep compiler-owned runtime roots separate from user-local
dependency roots and must reject aliases that shadow a selected runtime.

The first implementation slice establishes the manifest schema, defaults, and
target compatibility checks. Later slices add builtin root discovery,
canonical `std::` module mapping, reachability-based compilation, lockfile
identity, and `actus init` templates.

## Compatibility

- `core` is valid for hosted and freestanding targets.
- `std` is valid for hosted and freestanding targets. Its module capability
  declarations are checked against the selected target before resolution.
- `freestanding` is valid only for freestanding targets; a hosted target must
  not silently lose its host runtime contract.
- Omitting `runtime` is equivalent to `runtime = "core"`, preserving existing
  manifests and preventing an accidental standard-library dependency.

These rules are checked while loading the manifest. They are not repaired by
the code generator.

## Invariants

1. Runtime selection is represented by a closed, typed enum.
2. The default profile is `core` and introduces no filesystem dependency.
3. `std` resolution never depends on an absolute path written by the user.
4. Runtime roots are distinct from explicit local dependency roots.
5. Runtime selection is part of compiler configuration and later artifact and
   lockfile identity.
6. A runtime/target capability mismatch produces a deterministic configuration
   error before code generation.
7. Hosted-only modules are never made available to freestanding targets by
   selecting `std`.
8. Only modules reachable from source imports are compiled.
9. The profile does not weaken ownership, source-limit, or target contracts.
10. Runtime configuration failures use stable diagnostics for missing
    compiler-owned metadata, unsupported profile values, and
    runtime/target incompatibility. A builtin module that exists in the
    standard-library source tree but is unavailable for the selected target
    uses a distinct module diagnostic rather than a generic missing-facade
    error.

## Consequences

Positive consequences:

- a new project can opt into the standard library with one portable manifest
  field;
- existing manifests remain dependency-free by default;
- hosted and freestanding builds can share one standard-library package while
  receiving only target-valid modules;
- builtin runtime resolution has a clear boundary for compiler packaging and
  LSP integration.

Costs and risks:

- compiler distribution must carry or expose a versioned builtin/sysroot
  location;
- lockfiles and artifact metadata must include the selected profile and
  runtime version before reproducibility is complete;
- module resolver, CLI templates, LSP, formatter, and diagnostics need one
  shared runtime configuration contract.

## Non-goals

- This ADR does not add implicit third-party dependencies.
- This ADR does not make every `std` module available without an import.
- This ADR does not define package publishing or registry resolution.
- This ADR does not change target selection or entry-contract semantics.
- This ADR does not make hosted operating-system services available on
  freestanding targets.

## Acceptance criteria

- `runtime = "core"`, `"std"`, and `"freestanding"` parse into typed
  configuration values.
- Omitted runtime selects `core`.
- Invalid runtime/target capability combinations are rejected with stable
  diagnostics.
- Builtin `std` root discovery works without user-authored absolute paths.
- Canonical `std::io`, `std::fs`, and `std::path` imports resolve through the
- builtin facades and target capability metadata expose only target-valid
  modules and compile them only when reachable.
- A freestanding `std` project can compile target-neutral and embedded
  standard-library modules while rejecting hosted-only imports.
- Runtime profile and standard-library version participate in lockfile and
  artifact identity.
- `actus init` emits a portable manifest and a first valid build succeeds.
- Accepted, rejected, and native execution tests cover each profile.
