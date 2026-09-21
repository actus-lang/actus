# ADR-0001: Native Resource Handle and Cleanup ABI

- Status: Accepted for Alpha implementation
- Date: 2026-09-21

## Context

The native backend currently lowers only scalar `Int` values. Deterministic
cleanup plans already exist in the semantic model, but resource destruction
cannot be emitted until the backend has a stable representation for owned
resources and their destructors.

## Decision

Actus Alpha represents a native owned resource as an opaque, pointer-sized
resource handle. The handle is a value transferred between registers and
function parameters; copying it is not an ownership operation.

Each resource type supplies one destructor entry point known to the backend.
The destructor consumes the handle and is called exactly once for every live
owner. The backend emits destructor calls for explicit `drop`, normal lexical
scope exit, and control-flow unwinding.

Ownership operations follow these rules:

- `erg` owns the handle and may be moved or dropped.
- `dat` transfers the handle into the callee; the caller binding becomes
  `Moved` and receives no cleanup call.
- `abs` borrows the handle and never receives a destructor call.
- returning an owned handle transfers it to the caller and excludes it from
  callee cleanup.
- a moved or explicitly dropped handle is never destroyed again.

The compiler remains responsible for proving borrow and ownership safety. The
native backend only lowers validated cleanup actions and does not add a
runtime reference counter or infer ownership from machine code.

The concrete `Buffer` layout, allocator contract, and runtime destructor
symbols are separate decisions. This ADR defines the ownership ABI without
prematurely fixing a platform-specific data layout.

## Consequences

This contract permits deterministic cleanup lowering without exposing lifetime
annotations to users. It requires a target-aware pointer type in the native
backend and a small runtime contract for each resource type. Scalar `Int`
programs remain unchanged because they have no destructor entry point.
