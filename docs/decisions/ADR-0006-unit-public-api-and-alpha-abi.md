# ADR-0006: Unit Public API and Alpha ABI Policy

- Status: Accepted
- Date: 2026-09-22
- Scope: Actus unit interfaces and Arca compilation artifacts

## Context

Actus units need a precise public interface, but the Alpha compiler is still
evolving. Freezing an internal binary ABI too early would make compiler and
runtime development unnecessarily difficult. At the same time, unit
visibility, ownership roles, target compatibility, and cache invalidation
must remain deterministic.

## Decision

Actus separates the stable public source contract from the unstable Alpha
internal ABI.

## Public API

Only declarations marked `open` are exported across a unit boundary. All
other declarations are internal to their module and unit.

Every parameter type and return type in an exported verb must itself be an
`open` type. This rule applies recursively to types nested inside public
signatures and resource handles.

Ownership roles remain part of the public signature:

```act
open verb process(abs input: Buffer, dat payload: Packet) -> Result {
    ...
}
```

The `erg`, `abs`, and `dat` roles must be preserved in the unit interface.
Alpha's non-escaping borrow rule still prohibits returning an `abs` borrow,
even when the referenced type is public.

An exported declaration that refers to an internal type is rejected during
semantic analysis. Public API validation happens before code generation.

## Alpha Internal ABI

The Actus-to-Actus unit ABI is explicitly unstable during Alpha. Binary
compatibility is guaranteed only when all of the following match:

```text
compiler version
toolchain hash
target triple
```

The toolchain hash identifies the exact compiler build and relevant backend
toolchain. Matching a version string alone is insufficient. Any compiler,
backend, language, or ABI change may invalidate previously compiled Actus
units.

Actus must not describe this internal ABI as stable until a later architecture
decision establishes a compatibility contract.

## Stable C ABI Boundary

The C ABI is the only stable binary boundary in Alpha. It is used for:

- operating-system and system-library integration;
- hardware and device interfaces;
- external runtime components;
- embedded startup and platform support;
- interoperability with existing C libraries.

The unstable Actus unit ABI must not be confused with the stable C FFI
contract. C declarations and ownership rules remain subject to the separate
FFI architecture decisions.

## Unit Interface Metadata

Each compiled unit must produce a deterministic interface metadata artifact,
with `.actmeta` as the canonical Alpha file extension. The metadata describes
the interface and compatibility identity without embedding the unit's full
implementation.

The metadata must contain at least:

```text
unit_name
unit_version
compiler_version
toolchain_hash
target_triple
exported_symbols
unit_dependencies
```

Each exported symbol records its name, declaration kind, parameter types,
return type, and `erg`/`abs`/`dat` roles. The metadata representation must be
deterministic so identical inputs produce identical interface artifacts.

The exact serialized encoding may be selected during Arca implementation,
but it must remain versioned and independently parseable by the compiler and
package tooling.

## Incompatible Unit Handling

Arca or the compiler must reject a precompiled Actus unit before binary
linking when its `toolchain_hash` or `target_triple` does not match the
current build.

When compatible source is available, the package manager must rebuild the
unit with the current toolchain and regenerate its metadata and artifact. A
stale binary must never be silently linked.

When source is unavailable, the build fails with a deterministic diagnostic
that identifies the incompatible unit and the mismatched compatibility
fields.

The unit cache key must include the toolchain identity, target identity,
source inputs, unit dependencies, and relevant build settings.

## Current Alpha Status

This ADR defines the architecture and does not require immediate
implementation of `.actmeta`, stable unit linking, or public-type validation.
Those features must be implemented before Actus supports independently
compiled Actus units.

The current compiler remains a single Rust package and continues to use its
existing native backend and C FFI boundary.

## Consequences

Actus can evolve its compiler and internal representation without prematurely
freezing a binary contract. Public APIs remain explicit and type-safe, C
interoperability has a clear stable boundary, and Arca can reject stale
artifacts deterministically instead of producing unreliable links.

