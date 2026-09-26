# ADR-0007: Standard Library Layers and Operating Profiles

- Status: Accepted
- Date: 2026-09-22
- Scope: Actus standard library and target profiles

## Context

Actus must support both hosted applications and freestanding bare-metal
programs without embedding operating-system behavior in the compiler. The
standard library therefore needs explicit layers with predictable resource
requirements and a one-way dependency graph.

## Decision

Actus standard functionality is divided into three layers:

```text
core -> runtime -> std
```

Each layer may use only layers to its left. A higher layer must not be required
by a lower layer, directly or indirectly.

## `core`

`core` is the freestanding foundation. It must be usable without an
operating system, libc, or dynamic heap.

It provides the language and target primitives required by all programs,
including:

- fixed-width primitive types and boolean values;
- pointer, alignment, and layout primitives;
- volatile memory access;
- bit operations;
- panic and abort hooks;
- target-provided low-level contracts.

`core` must not provide filesystems, networking, console I/O, threads, or a
heap allocator. `core` cannot depend on `alloc` or `std`.

### Panic Hook Contract

`core` panic is a low-level divergence or trap hook. The target or selected
runtime defines its concrete behavior, such as an infinite loop on bare metal
or process termination in a hosted environment.

The core API must not assume a specific panic reaction. Panic behavior belongs
to the target/runtime boundary and must remain deterministic for that target.

## `runtime`

`runtime` supplies target-selected capabilities while remaining outside the
language foundation. A hosted runtime may provide heap-backed buffers and
operating-system bridges; a bare-metal runtime may provide static memory or
target-specific device services.

It may provide explicit contracts for:

- allocation and release;
- owned resource layouts;
- console or device I/O;
- panic, startup, and target hooks.

`runtime` depends on `core` contracts and must expose its operations through
explicit ABI boundaries. A freestanding target may omit host-only runtime
capabilities entirely.

## `std`

`std` is the hosted system layer and depends on `core` and selected `runtime`
capabilities.

Its initial responsibilities include:

```text
std/
├── mem/
├── io/
├── fs/
└── net/
```

This layer may provide filesystem access, console I/O, networking, process
services, and hosted task support. `std` is unavailable to a freestanding
profile unless a target explicitly supplies an equivalent hosted contract.

## Operating Profiles

Library availability is selected by the package manifest and target profile,
not by source-level magic annotations:

```text
freestanding:    core
runtime-enabled: core + runtime
hosted:          core + runtime + std
```

The exact Actus manifest spelling is a separate build-system decision, but the
selected profile must be explicit and recorded in build metadata.

The compiler must reject imports or operations requiring unavailable layers
before code generation.

## Compiler Boundary

The compiler binary must not contain implementations of standard-library
services. It may know about intrinsic names, types, and semantic contracts,
but their implementations belong to `core`, `runtime`, `std`, or the selected
target.

Lowering to an intrinsic or runtime operation is valid only after the
frontend has verified the selected profile and required layer.

## Consequences

This model supports bare-metal programs without an operating system or heap,
while hosted programs receive richer services through explicit layers. The
one-way dependency graph prevents platform functionality from leaking into
the language foundation and keeps the compiler independent of library
implementations.
