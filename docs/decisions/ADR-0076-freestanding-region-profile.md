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

Keep `std::region` hosted-only until a complete freestanding profile exists.
The profile must define a target-compatible resident-window provider, capability
allocation and generation rules, cleanup ownership, failure mapping, and a
native bridge that has no libc, filesystem, hosted allocator, or hosted Region
runtime dependency.

The public source contract should remain compatible with the hosted facade:
`Region[T]`, checked window access, explicit publication/cancellation, close,
typed errors, and Actus ownership roles. The provider and bridge may differ by
target, but the source-level ownership and bounds invariants must not weaken.

The standard-library registry may list `freestanding` for `std::region` only
after the freestanding bridge and object tests pass. Registry-only enablement is
rejected because it would make source resolution succeed while leaving hosted
runtime symbols or unsupported cleanup behavior at the object boundary.

The compiler may nevertheless lower a freestanding `Region[T]` owner cleanup
boundary before the facade is enabled. That object may import only the
target-provided `actus_region_drop` symbol; it must not acquire the hosted
runtime archive. This staged compiler boundary does not make Region operations
available on freestanding targets.

## Required implementation gates

1. Specify the no-host resident-window provider and its bounded storage budget.
2. Implement target-compatible capability and generation management.
3. Implement private freestanding Region bridges with the hosted error domain.
4. Add semantic, object, and native target tests for logical scale and cleanup.
5. Prove absence of hosted runtime, filesystem, libc, and raw OS pointer imports.
6. Publish dated freestanding evidence separately from hosted and simulated data.

## Consequences

Hosted Region behavior remains unchanged and keeps its current evidence. A
freestanding build fails early and clearly until the required profile is ready.
This preserves the target boundary and prevents a false claim that a hosted
in-memory backend is usable on bare metal.
