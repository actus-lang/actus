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
core -> alloc -> std
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

## `alloc`

`alloc` adds heap-backed functionality while remaining independent of any
specific operating system.

It may provide:

- an allocator interface;
- owned heap containers;
- dynamic arrays;
- strings and other heap-backed values.

`alloc` depends only on `core`. It requires an allocator contract supplied by
the selected program, target, or runtime. A freestanding program may provide
a custom allocator, static memory pool, or RTOS allocator; allocation is not
implicitly guaranteed merely because `alloc` is available.

## `std`

`std` is the hosted system layer and depends on both `core` and `alloc`.

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
freestanding: core
alloc-enabled: core + alloc
hosted:        core + alloc + std
```

The exact Actus manifest spelling is a separate build-system decision, but the
selected profile must be explicit and recorded in build metadata.

The compiler must reject imports or operations requiring unavailable layers
before code generation.

## Compiler Boundary

The compiler binary must not contain implementations of standard-library
services. It may know about intrinsic names, types, and semantic contracts,
but their implementations belong to `core`, `alloc`, `std`, or the selected
target runtime.

Lowering to an intrinsic or runtime operation is valid only after the
frontend has verified the selected profile and required layer.

## Consequences

This model supports bare-metal programs without an operating system or heap,
while hosted programs receive richer services through explicit layers. The
one-way dependency graph prevents platform functionality from leaking into
the language foundation and keeps the compiler independent of library
implementations.

