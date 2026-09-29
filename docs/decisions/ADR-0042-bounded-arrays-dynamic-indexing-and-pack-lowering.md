# ADR-0042: Bounded Contiguous Arrays, Dynamic Indexing, and Native Pack Mutation

- Status: Accepted
- Date: 2026-09-29
- Scope: bounded contiguous storage, dynamic indexing, and native packed-field lowering

## Context

Actus already provides fixed-width primitive values, packed layouts, and
lexically scoped arenas. These facilities are sufficient for statically named
fields and arena placement, but they do not yet form a complete primitive for
bounded indexed storage.

Without an indexed storage contract, programs must encode a fixed set of
bindings instead of traversing a contiguous region through a runtime index.
Without complete native pack reads and writes, packed fields cannot serve as a
general data representation: source code can describe a layout, but native
lowering cannot consistently load, update, and preserve individual fields.

The missing capabilities must be added as orthogonal language and backend
primitives. They must not be introduced as application-specific exceptions,
hidden pointer arithmetic, or compiler-generated copies of an entire storage
region.

## Decision

Actus will define a bounded contiguous indexed type using the form
`Array[T, N]`, or an equivalent canonical spelling approved by the language
grammar. `N` is a compile-time capacity and the storage consists of exactly
`N` elements of `T` in contiguous target-defined layout.

The indexed type will provide checked dynamic indexing:

- a read evaluates `array[index]` without copying the complete array;
- a write evaluates `array[index] = value` in place;
- the index is checked against the declared capacity before memory access;
- an out-of-bounds access produces a defined runtime failure or typed result,
  according to the selected API contract, and never performs an unchecked
  load or store;
- constant indices may be rejected at compile time when they are provably
  outside the range `[0, N)`.

The capacity `N` is a non-negative decimal integer in the source grammar, but
the semantic contract admits only positive capacities. `Array[T, 0]` is
rejected before layout because it has no indexable slot and would otherwise
require a target-specific zero-sized representation. Future zero-sized
storage requires a separate decision rather than inheriting this contract.

Array storage may be owned directly by an `erg` binding, embedded in an arena
allocation, or exposed through a documented borrowed view. The representation
must remain contiguous in all supported storage locations.

## Ownership and slot access

The array itself remains the owner of its elements. An element read produces a
value according to the element type's normal ownership contract; it does not
silently create a longer-lived borrow.

An exclusive slot operation may use an `ins` contract. The slot loan is tied to
the source array and call scope:

- the array owner is suspended for the duration of the loan;
- the callee receives exclusive access to one validated slot;
- no second alias to that slot may be created while the loan is active;
- the slot view cannot escape the call or outlive the source array;
- the operation mutates the original storage and does not copy the entire
  array;
- a live slot loan prevents moving, dropping, or relocating the source array.

An `abs` indexed view is read-only and non-owning. It preserves single-origin
provenance and cannot be stored in a longer-lived owner. A `dat` operation on
an array transfers the complete array ownership and invalidates the caller's
binding. `erg` remains the only role that can directly mutate an owned array
outside an exclusive `ins` call.

## Layout and placement contract

The layout of `Array[T, N]` is deterministic:

- elements occupy `N * sizeof(T)` bytes, subject to checked overflow;
- element alignment is the target alignment of `T`;
- element `i` has byte offset `i * sizeof(T)`;
- padding, if required by the target ABI, is specified by the array layout
  contract and cannot be inferred differently by frontends or backends;
- zero-length arrays, if admitted by the language, have an explicit layout and
  indexing rule rather than relying on a null or dangling pointer convention.

For the accepted contract, zero-length arrays are not admitted. Accepted
arrays have no inter-element padding, use the alignment of `T`, and publish a
layout only after checking multiplication overflow in `N * sizeof(T)`. Any
ABI tail padding belongs to the containing aggregate and does not change
element offsets. Stack, struct, and arena placement use the same layout
record.

Arena placement preserves the same element order, alignment, capacity, and
provenance rules. Arena reset or destruction is rejected while an `abs` view or
`ins` slot loan remains live.

## Bounds-check contract

Bounds validation is part of the language operation, not an optional library
helper. The compiler may eliminate a check only when it proves the same
capacity invariant for the access. Such elimination must preserve observable
failure behavior for all inputs that are not proven safe.

The failure contract must be stable across debug, release, native, and
freestanding targets. It must not become an unchecked access because a target
does not provide a hosted panic or exception runtime. A target may select its
own failure mechanism only through an explicit target contract.

The first native contract is a deterministic non-returning bounds trap. A
dynamic out-of-bounds access lowers to Cranelift's `HEAP_OUT_OF_BOUNDS` trap;
hosted targets surface it as the normal non-zero process failure, while
freestanding targets retain the trap without requiring a host runtime.
Constant violations remain semantic errors and never reach code generation.

## Existing pack implementation baseline

The compiler already validates pack storage width, field width and role,
bit-range coverage, overlaps, reserved regions, and duplicate names during
semantic analysis. The Cranelift layout registry records backing storage,
endianness, field offsets, widths, and native field types. This is an
inventory baseline only: complete native field reads, clear-and-insert writes,
signed extraction, and array/slot composition remain Gate 4 responsibilities.

## Native pack lowering

ADR-0026 is extended so every validated packed field has a complete native
read and write path.

For a field with bit offset `o`, width `w`, and backing integer width `b`:

- reads lower to a shift and mask operation using the pack's declared
  endianness and the target integer representation;
- writes lower to clear-mask and insert operations that preserve all unrelated
  backing bits;
- signed fields apply sign extension only after extraction and only when the
  declared field type is signed;
- writes validate the source range before truncation and reject overflow;
- overlapping fields and fields outside the backing capacity remain rejected
  during layout validation;
- no user-facing Actus code is required to reproduce these bit operations by
  hand;
- the backend must lower field access to native IR rather than route it through
  a hidden heap object or a general-purpose runtime allocation.

Pack field access must work for local values, array elements, arena slots, and
`ins` slot loans. A failed lowering lookup is a compiler error; it must never
produce a default scalar, zero value, or silently unavailable field.

## Pipeline and ABI boundaries

The feature follows the existing one-directional pipeline:

```text
lexer -> parser -> ast -> semantic -> codegen
```

The parser records indexing and packed-field syntax only. Semantic analysis
validates element types, index types, bounds that are statically knowable,
ownership roles, slot provenance, and field ranges. Codegen lowers validated
operations to target IR and must not repair semantic failures.

Indexed access is an internal Actus operation unless an explicit C ABI view is
declared. C ABI bridges must receive a documented pointer, element size,
capacity, and lifetime contract. They must not receive an unchecked index or
an implicit pointer to an array that may be relocated.

## Invariants

1. `Array[T, N]` storage is contiguous, bounded, and deterministically laid
   out.
2. Every dynamic index is validated before memory access.
3. A slot `ins` loan is exclusive, non-escaping, and does not copy the array.
4. Array relocation, move, drop, and arena reset respect live view and loan
   records.
5. Packed reads and writes preserve unrelated backing bits.
6. Packed field values cannot overflow their declared widths silently.
7. No indexed or packed operation inserts a hidden heap allocation.
8. No backend fallback may conceal an unresolved array layout or pack field.
9. Zero floating-point operations are required for these primitives unless a
   user program explicitly selects a floating-point element or field type.
10. All target-specific failure behavior is explicit and deterministic.

## Implementation order

1. Add AST and parser representation for the canonical bounded array type and
   indexed expressions.
2. Define semantic element, index, capacity, ownership, and slot-loan rules.
3. Add accepted and rejected fixtures for constant and dynamic bounds.
4. Extend layout computation for stack, struct, and arena-resident arrays.
5. Implement complete native pack read/write lowering with range checks.
6. Implement dynamic array load/store lowering with target failure paths.
7. Add `ins` slot access without whole-array copies and verify provenance.
8. Add native execution tests for arrays, arena slots, packed fields, and
   failure behavior.
9. Verify deterministic artifacts and cross-target behavior before updating
   the roadmap.

## Acceptance criteria

- Valid fixed-capacity arrays compile with deterministic layout metadata.
- Valid dynamic reads and writes execute against the selected slot.
- Constant and runtime out-of-bounds indices are rejected through stable
  diagnostics or the documented runtime failure contract.
- `ins` slot mutation updates the source array without copying the complete
  backing storage.
- Pack fields can be read and written in locals, arrays, arenas, and slot
  loans while preserving unrelated bits.
- Signedness, width, endianness, and overflow behavior have positive and
  negative tests.
- Native and freestanding targets use explicit, deterministic bounds-failure
  behavior.
- Source limits, architecture checks, documentation checks, and the full
  quality matrix pass without undocumented exceptions.

## Consequences

This decision adds a general systems primitive that can support parsers,
device tables, schedulers, bounded queues, arenas, and other deterministic
data structures without introducing a general-purpose heap abstraction.

The compiler and runtime must carry more explicit layout and provenance
metadata. Bounds checks and slot loans add semantic and code-generation work,
but they prevent unchecked pointer arithmetic and make in-place mutation
reviewable. Existing packed declarations remain valid; declarations that lack
complete native lowering must remain rejected until their lowering contract is
implemented.

## Implementation Evidence

The first native execution milestone is implemented and verified in the
compiler and integration test suite:

- bounded arrays use contiguous stack or arena storage with dynamic bounds
  checks and deterministic out-of-bounds traps;
- indexed reads and writes lower to direct element addresses, without copying
  the complete array;
- packed fields lower to native extraction and insertion operations while
  preserving unrelated backing bits;
- an `ins` indexed argument passes the caller-owned slot address directly;
- a scalar `ins` argument is materialized in a caller-owned stack slot before
  the call, preserving the existing typed ABI for buffer and wide-integer
  values;
- repeated native builds and executions are covered by deterministic artifact
  and exit-code tests.

## Approved Implementation Boundary

The current native lowering accepts scalar `ins` identifiers and indexed slots
as addressable call arguments. A scalar identifier is materialized into a
stable call-scope slot; indexed slots use their existing array address. The
semantic layer remains authoritative for ownership and lifetime validation.
The code generator rejects unsupported non-addressable `ins` expressions with
an explicit error instead of silently copying them. Full SSA-to-storage
write-back for scalar locals after arbitrary nested calls remains a separate
ABI/storage milestone; indexed slots already mutate their source storage
directly and therefore provide the required caller-visible behavior.

## Checked Primitive Casts and Index Type Orthogonality

Primitive conversion is explicit and checked. The source spelling is
`expression as Type`; conversions are represented in the AST and validated by
semantic analysis before code generation. Implicit conversion between integer,
buffer, pointer-like, and unrelated primitive types is forbidden.

Array and buffer indexing accepts the signed language integer `Int` and the
unsigned index types `u8`, `u32`, and `Usize`. The index is converted to the
target machine index width only after its type has been validated, and the
ordinary capacity or buffer-length check remains mandatory.

Constant casts are range-checked during semantic analysis. Non-constant integer
casts emit a native runtime range check and trap deterministically on overflow
or underflow. A cast never changes ownership, creates an allocation, or hides
an ABI boundary.

## Non-goals

- This ADR does not define an unbounded dynamic array or general allocator.
- This ADR does not introduce garbage collection or reference counting.
- This ADR does not change Actus ownership roles or relax non-escaping views.
- This ADR does not define a domain-specific execution engine or application
  model.
- This ADR does not permit user code to bypass bounds checks through ordinary
  indexing syntax.
