# ADR-0076: Freestanding Region Profile

- **Status:** Proposed
- **Date:** 2026-10-07
- **Scope:** Target availability, resident-window providers, and native object
  boundaries for `std::region` on freestanding targets

## Context

The hosted `std::region` implementation is now validated for bounded logical
capacities through 1 TiB. Its private bridges use the hosted Rust runtime's
resident store and capability lifecycle. A freestanding target cannot link
those hosted services and must not gain access to them by registry metadata
alone.

The first freestanding probe used `target = "x86_64-unknown-none"` and
`import std::region`. Actus correctly rejected it before native emission with
`E1112`: the builtin module is incompatible with the configured target and its
facade is unavailable.

## Decision

Expose `std::region` to freestanding source only through an explicit
target-provider ABI. The complete freestanding profile must still define a
target-compatible resident-window provider, capability allocation and
generation rules, cleanup ownership, failure mapping, and a native bridge that
has no libc, filesystem, hosted allocator, or hosted Region runtime dependency.

The public source contract should remain compatible with the hosted facade:
`Region[T]`, checked window access, explicit publication/cancellation, close,
typed errors, and Actus ownership roles. The provider and bridge may differ by
target, but the source-level ownership and bounds invariants must not weaken.

The standard-library registry lists `freestanding` for `std::region` only after
the facade object was proven to import the target-provider bridge symbols
without hosted runtime services. Registry-only enablement remains rejected:
source resolution is valid only because the bridge and object regression test
establish the required symbol boundary. A target must still provide those
symbols before a freestanding executable can link or run.

The compiler lowers a freestanding `Region[T]` owner cleanup boundary without
acquiring the hosted runtime archive. A cleanup-only object may import only the
target-provided `actus_region_drop` symbol. This staged compiler boundary does
not by itself provide a resident-window implementation.

The current compiler-level stage closes only the bridge/object boundary: a
strict `x86_64-unknown-uefi` object using caller-provided buffers, the complete
Region lifecycle, and a 1 TiB logical length imports only the nine target
provider symbols: the seven `actus_region_*` operations plus
`actus_buffer_drop` and `actus_enum_drop`. The target-provider implementation,
executable link, and resident-window execution remain open acceptance work.

## Required implementation gates

1. Specify the no-host resident-window provider and its bounded storage budget.
2. Implement target-compatible capability and generation management.
3. Implement private freestanding Region bridges with the hosted error domain.
4. Add semantic, object, and native target tests for logical scale and cleanup.
5. Prove absence of hosted runtime, filesystem, libc, and raw OS pointer imports.
6. Publish dated freestanding evidence separately from hosted and simulated data.

## Consequences

Hosted Region behavior remains unchanged and keeps its current evidence. A
freestanding object can now express the source contract and link against an
explicit target provider, while a freestanding executable remains incomplete
until that provider is supplied and tested. This preserves the target boundary
and prevents a false claim that a hosted in-memory backend is usable on bare
metal.
