# ADR-0046: Struct Ownership, Visibility, and Representation

- Status: Proposed
- Date: 2026-09-29
- Scope: Alpha struct field visibility, ownership, moves, references, layout,
  cleanup, and public ABI

## Context

Actus already parses and lowers structs, nested aggregates, field access,
field assignment, generic struct instances, deterministic cleanup, and
caller-allocated struct returns. The implementation has therefore passed the
first structural milestones, but several contracts remain implicit. In
particular, the language does not yet provide one authoritative answer for
field visibility, field-level ownership, aggregate copy, assignment after a
partial move, reference-bearing fields, packed representation, or public
struct ABI exposure.

Implicit behavior is unsafe at the Alpha boundary. A field can own a resource,
borrow a resource, or merely contain scalar data, and those cases require
different cleanup and assignment rules. Likewise, a native layout can be
correct for an internal Actus expression while still being unsuitable for a
public C ABI. These concerns must be separated before standard-library
applications depend on them.

This ADR defines the contract for Phase 19 Gate 19.1. It does not claim that
all rules are already implemented. The implementation gates must add semantic
validation, native lowering, diagnostics, tests, and documentation in the
order defined here.

## Decision

### 1. Struct visibility

`open` remains the only Alpha visibility modifier. Visibility applies to the
struct declaration as a module API boundary; there is no independent `pub`,
`closed`, or field-level visibility keyword in Alpha.

- A declaration without `open` is private to its declaring module.
- An `open struct` makes the type name available to importing modules.
- Fields of an `open struct` are available to importing modules only through
  the field access rules and ownership roles defined below.
- A private struct cannot appear in an `open` verb signature, public role
  contract, public enum payload, or public standard-library declaration.
- Visibility never grants mutation, ownership, or a lifetime extension.
- An imported module may not bypass a private struct through a literal,
  field-access expression, generic argument, or inferred type.

The compiler must diagnose a private-type leak at the declaration that exposes
it. It must not wait for code generation or repair the signature by making the
type public.

### 2. Field categories and ownership

Struct fields have four Alpha categories in the AST. The categories are not
interchangeable and are preserved through semantic analysis, layout, cleanup,
generic specialization, and native lowering.

#### 2.1 Value fields

A field without an ownership marker is a `Value` field.

- Scalar values are stored directly in the containing aggregate.
- Aggregate values are part of the containing owner's move and cleanup plan.
- A value field does not create an independent owner binding.
- Field access follows the containing expression's access state.
- Assignment must preserve the declared field type and must account for any
  existing owned payload before replacement.

Value fields are the default representation for passive data. They do not
silently become borrowed or reference-counted fields because their type is
large, pointer-shaped, or externally represented.

#### 2.2 `erg` fields

An `erg` field is an explicitly owned mutable subresource.

- The containing struct is responsible for the field's cleanup unless the
  field is moved out through an explicit ownership operation.
- `erg` field assignment requires a mutable owner access path.
- Moving an `erg` field marks that field path as moved and leaves unrelated
  fields live.
- A struct with partially moved `erg` fields has a partial cleanup plan; it
  must never drop the moved field twice.
- A frozen or suspended containing owner cannot mutate, move, or drop an
  `erg` field.

#### 2.3 `abs` fields

An `abs` field is a non-owning read-only view stored with explicit provenance.

- It never has destructor responsibility.
- It may be read only through an active view of the containing struct.
- Its source origin must remain live and cannot be moved, mutated, or dropped
  while the field view is live.
- Construction must establish a valid single-origin borrow record.
- The view cannot escape its source scope or be returned as an unrelated owner.
- An `abs` field may not be used to create an `erg`, `dat`, or `ins` binding.
- An `abs` field may be present in standard-library view aggregates only when
  the existing single-origin and non-escaping borrow rules prove its safety.

The Alpha implementation does not infer a general lifetime relationship from
an `abs` field. If the source and destination scopes cannot be proven by the
existing origin model, construction is rejected.

#### 2.4 `ins` fields

An `ins` role is a temporary exclusive call-scope loan, not persistent field
ownership. Alpha therefore rejects `ins` fields in struct declarations.

- An `ins` value may be passed as a parameter or indexed slot loan.
- It may not be stored in a struct, enum payload, array element, or returned
  aggregate.
- The caller's owner must be restored when the loaned call ends.

This restriction prevents a temporary access state from becoming an
undocumented long-lived layout property.

#### 2.5 `dat` and fields

`dat` is an operation-level ownership transfer, not a persistent struct-field
category. Alpha does not add `dat` fields. A move from a value or `erg` field
must be expressed by an operation whose semantic result is a transfer; the
field declaration itself does not encode a destination ownership state.

### 3. Aggregate copy, move, and assignment

Alpha structs are not implicitly `Copy`. A struct value is transferred by an
explicit ownership operation or borrowed through `abs`.

- Binding an owned struct to another owned binding transfers ownership and
  invalidates the source according to normal `dat` rules.
- Passing an owned struct to a `dat` parameter transfers the aggregate.
- Passing an owned struct to an `abs` parameter creates a lexical frozen view.
- Passing an owned struct to an `ins` parameter creates a temporary exclusive
  loan and restores the caller afterward.
- A struct literal constructs a new aggregate; it does not copy another
  aggregate implicitly.
- Whole-struct assignment is allowed only for a live mutable destination and
  a type-compatible source. The destination's previous owned fields are
  cleaned up before replacement.
- Assignment must not convert a frozen, suspended, moved, or dropped value
  into an active owner.
- Partial field moves are tracked by field path and are joined only when all
  control-flow branches agree on the resulting ownership state.

Future explicit clone or copy contracts may define opt-in behavior. They are
outside this ADR and must not be inferred from bitwise layout or trivial native
copyability.

### 4. Reference-bearing and recursive fields

Alpha permits recursive type references only through the already-supported
bounded, provenance-aware reference representations. It does not permit an
arbitrary reference field to escape lexical proof.

- An `abs` reference field requires a live single-origin source.
- A reference field may not outlive its source owner or arena.
- A reference may not be stored in a longer-lived owner when its origin is a
  local binding, temporary, or unresolved external source.
- Self-referential ownership without an explicit arena/provenance contract is
  rejected before code generation.
- Cross-arena references and mixed-origin aggregate construction are rejected.
- Recursive definitions must remain layout-finite or use the existing
  pointer-niche representation with explicit `Option` ownership semantics.

The compiler must report the field, source origin, and blocking scope in a
stable diagnostic. A backend must never accept a representation that semantic
analysis rejected as escaping.

### 5. Layout and representation

The semantic type and field role determine layout inputs; the backend chooses
the target-native machine representation from the validated layout registry.

- Field declaration order is semantic and determines logical layout order.
- Each field receives a deterministic offset, size, alignment, and ownership
  cleanup classification.
- Natural structs use target-native alignment and padding documented by the
  selected target contract.
- Packed bit layouts use `pack` declarations and ADR-0026 rules; an ordinary
  struct must not be silently packed.
- `abs` fields use the documented borrowed-view representation and do not add
  destructor work.
- `erg` fields retain the ownership metadata required for deterministic drop.
- `ins` fields do not have a layout because they are rejected as declarations.
- Generic struct layouts are specialized only after field roles and types are
  validated.
- Layout identifiers, field offsets, and aggregate sizes must be deterministic
  across repeated compilations for the same target contract.

The code generator consumes `StructLayout` and cleanup metadata. It must not
infer visibility, ownership, packing, or lifetime validity from a raw field
type or pointer shape.

### 6. Public C ABI

Alpha keeps the C ABI boundary explicit and conservative.

- A public struct may cross the C ABI only when its representation is fully
  specified by an accepted target contract.
- Structs containing `abs` or other non-owning internal views are not exported
  as public C aggregates unless a dedicated ABI record defines the view
  representation and lifetime contract.
- Structs with `erg` fields are not exported as opaque ownership-free C values
  without an explicit destructor and transfer contract.
- `pack` values may cross the boundary only through their documented backing
  integer representation and endian contract.
- Aggregate returns use the caller-allocated `sret` convention from ADR-0031.
- Raw pointers, hidden ownership flags, and host-specific padding must not be
  exposed accidentally through a public declaration.
- Unsupported public layouts are rejected during semantic/configuration
  validation before native linking.

Until a public aggregate ABI is explicitly accepted, Alpha supports structs as
Actus-internal values and exposes C-compatible scalar/packed bridges or
explicit pointer contracts instead.

### 7. Diagnostics and phase ownership

The lexer and parser recognize declarations and expressions only. They do not
decide whether a field ownership transition is valid.

The semantic phase owns:

- visibility leakage checks;
- field-role validation;
- copy/move/assignment state transitions;
- borrow-origin and escape checks;
- partial-move joins;
- public ABI eligibility.

The code generator owns:

- target-specific layout lowering;
- field address/load/store operations;
- deterministic cleanup emission;
- caller-allocated aggregate return lowering.

Diagnostics remain renderer-independent and use stable `E####` codes. Codegen
must not repair an invalid field operation or silently insert a copy.

## Invariants

1. Every struct field has exactly one validated category: `Value`, `erg`, or
   `abs`; `ins` fields are rejected and `dat` is operation-level only.
2. A frozen or suspended owner cannot be mutated, moved, or dropped through a
   field path.
3. A moved field is excluded from cleanup exactly once.
4. An `abs` field cannot outlive its single-origin source.
5. A struct layout is deterministic for a fixed target contract.
6. Ordinary structs are not silently packed or exposed as C aggregates.
7. Public aggregate returns use ADR-0031 caller-allocated storage.
8. Semantic rejection occurs before code generation and linking.
9. No implicit copy, clone, ownership transfer, or lifetime extension is added
   by field access.

## Verification plan

Gate 19.1 must add the following evidence before it is complete:

- accepted and rejected visibility leakage programs;
- accepted `Value`, `erg`, and `abs` field programs;
- rejection of `ins` field declarations and invalid `dat` field syntax;
- field mutation through mutable owners and rejection through frozen owners;
- whole-struct move, partial field move, replacement assignment, and cleanup;
- overlapping move and branch-join diagnostics;
- escaping, self-referential, cross-arena, and mixed-origin reference cases;
- deterministic natural layout and nested cleanup tests;
- sret and public-ABI rejection/acceptance tests for supported representations;
- native execution tests proving field values and cleanup behavior.

All tests belong under the repository test tree. Struct implementation modules
must not contain embedded test fixtures.

## Consequences

This decision makes struct behavior more explicit and temporarily rejects
convenient but unsafe representations, especially persistent `ins` fields and
unproven public aggregate ABIs. It requires additional semantic state for
field paths and partial cleanup, but it prevents backend-specific guesses from
becoming language behavior.

The resulting Alpha struct model is intentionally conservative: internal
natural structs, explicit ownership, lexical views, deterministic layout, and
documented aggregate returns are supported; general lifetimes, implicit copy,
and opaque public aggregates remain future design work.

## Non-goals

- General lifetime parameters or unrestricted reborrowing.
- Implicit `Copy`, `Clone`, reference counting, or garbage collection.
- Persistent `ins` fields or implicit mutable iteration.
- ABI stability for unsupported or host-dependent aggregate layouts.
- Editor-side struct ownership analysis.
- A new ownership role or a replacement for `erg`, `abs`, `dat`, and `ins`.
