# Phase 11: Complete Struct System

Structs are intentionally deferred until the ownership, borrowing, and cleanup foundations are stable. This phase must implement structs as a complete language and compiler feature, not as parser-only syntax.

## Syntax and AST

- [x] Define struct declaration grammar and source-span rules.
- [x] Parse named fields with explicit types.
- [x] Parse struct literals and field initialization.
- [x] Parse field access and field assignment.
- [x] Represent structs and fields in the AST with dedicated modules.
- [x] Reject duplicate struct and field names.
- [x] Reject unknown fields and missing required fields.

## Semantic Model

- [x] Add a type environment for struct declarations.
- [x] Validate field types and recursive type references.
- [ ] Define field visibility and access rules.
- [ ] Define ownership semantics for `erg`, `abs`, and `dat` fields.
- [x] Define move semantics for whole structs and individual fields.
- [x] Reject partial use after moving a field.
- [x] Define and enforce struct initialization invariants.
- [ ] Define copy, move, and assignment behavior explicitly.

## Borrowing and Lifetimes

- [x] Define whether Alpha structs may contain `abs` fields.
- [ ] Reject self-referential and escaping borrow fields unless a lifetime model exists.
- [x] Reject storing lexical borrows in longer-lived structs.
- [x] Validate borrow access through struct fields.
- [x] Track field-level borrow records where supported.
- [x] Add diagnostics identifying the struct field and blocking borrow.

## Layout and Destruction

- [x] Define deterministic field declaration order.
- [x] Define size, alignment, and padding rules.
- [ ] Define packed and externally represented struct policies.
- [x] Generate field-level cleanup in reverse declaration order.
- [x] Handle moved and dropped fields without double cleanup.
- [x] Add layout and destruction golden tests.

## Methods and Backend

- [x] Define struct method syntax and receiver roles.
- [x] Validate receiver ownership and borrow behavior.
- [ ] Emit native struct layouts and declarations for the selected backend.
- [x] Emit field access and initialization.
- [x] Emit field assignment and cleanup.
- [ ] Define the C ABI contract for public structs.
- [x] Add generated native output and execution tests.

## Documentation and Compatibility

- [ ] Document the complete struct model and unsupported cases.
- [ ] Add architecture decision records for layout and field ownership.
- [ ] Add migration rules if struct semantics evolve after Alpha.
- [ ] Keep the manifesto, specification, and implementation behavior synchronized.
