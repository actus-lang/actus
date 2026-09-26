# ADR-0022: Runtime Layer and C FFI Contract

- Status: Accepted
- Date: 2026-09-26
- Scope: Phase 16 Gate 1

## Context

Actus needs host services for the first standard-library modules without
putting operating-system behavior into the compiler core or making hosted
services mandatory for bare-metal targets. The language already has an
explicit `unsafe extern "C"` boundary and a native runtime archive; this ADR
turns those existing mechanisms into one documented contract.

## Decision

The dependency direction is strictly:

```text
core -> runtime -> std
```

`core` defines language and target-independent contracts. It must not depend
on an operating system, filesystem, allocator, or hosted runtime service.
`runtime` supplies selected target capabilities through explicit ABI symbols.
`std` is Actus-facing library code built on those runtime capabilities. The
compiler may implement these contracts in Rust during bootstrap, but compiler
internals are not an application-level standard-library dependency.

The selected `TargetSpec` and entry contract control availability. Hosted
targets link the hosted runtime archive. Freestanding targets do not link it;
programs requiring a host capability therefore fail at the explicit target or
link boundary rather than silently acquiring an allocator or operating system.

## C ABI contract

Runtime declarations must use `unsafe extern "C"` and an explicit return type.
The supported Actus-to-C mappings are:

| Actus type | C representation |
| --- | --- |
| `Int` | `int32_t` |
| `Buffer` | `void*` to an `ActusBuffer` handle |

`ActusBuffer` is a `repr(C)` record containing `data`, `length`, and
`capacity`. Its layout is target-pointer-width dependent and is not an
ownership mechanism by itself.

Actus roles remain semantic obligations at every call boundary:

| Actus role | C ABI meaning for `Buffer` |
| --- | --- |
| `erg` | exclusive active owner |
| `abs` | non-owning shared view |
| `dat` | consumed owner; cleanup responsibility transfers |
| `ins` | exclusive call-scope loan; owner resumes after return |

The semantic analyzer rejects role/type combinations that cannot be expressed
by this ABI. It also rejects aliasing before native lowering. The code
generator emits the pointer representation directly; no wrapper object,
reference count, or hidden allocation is introduced.

## Runtime symbols and status behavior

The bootstrap hosted runtime exports a versioned symbol set:

```text
actus_buffer_allocate(usize) -> void*
actus_buffer_append(void*, u8) -> bool
actus_buffer_drop(void*) -> void
actus_print_int(int32_t) -> int32_t
actus_print_string(const uint8_t*) -> int32_t
```

Allocation failure returns a null handle. Buffer append returns `false` and
preserves the handle on invalid input or allocation failure. Null-safe release
is permitted, but releasing a live handle twice is invalid. Print operations
return a deterministic scalar result; they do not transfer ownership.

The runtime contract has an explicit ABI version and stable symbol constants.
Implementations for another target must provide the same contract or select a
different capability set in the target specification.

## Consequences

- Host services are visible at the FFI boundary and are excluded from
  freestanding links.
- Ownership remains a compile-time Actus responsibility before and after a
  runtime call.
- Runtime allocation is confined to the hosted runtime implementation; core
  contracts do not require it.
- Adding a runtime operation requires an ABI mapping, positive and negative
  semantic tests, and a native link or target-capability test.

## Verification

The contract is verified by FFI signature tests, invalid ownership tests,
borrowed and owned buffer return tests, runtime symbol tests, buffer native
execution tests, C library link tests, and freestanding target configuration
tests.
