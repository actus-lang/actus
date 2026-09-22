# Phase 5: Semantic Analyzer

## Bindings and Scopes

- [x] Implement lexical scope frames.
- [x] Implement binding tables.
- [x] Reject use of undeclared identifiers.
- [x] Reject duplicate bindings within the same scope.
- [x] Define and enforce shadowing rules.
- [x] Track binding source spans.
- [x] Validate known typed initializers and assignments.

## Ownership

- [x] Implement `erg` owner bindings.
- [x] Implement `dat` ownership transfer at call sites.
- [x] Transition moved caller bindings to `Moved`.
- [x] Reject use after move.
- [x] Reject use after explicit drop.
- [x] Reject double drop.
- [x] Define ownership transfer for returned values.
- [x] Validate known return expression types against verb declarations.
- [x] Enforce required return values for typed verb declarations.
- [x] Move returned local owners out of their scope.
- [x] Reject borrowed returns in Alpha.

## Borrowing

- [x] Implement `BorrowRecord` with identity, owner, scope, and origin span.
- [x] Implement `abs x = ref owner`.
- [x] Derive `Frozen` from active borrow records.
- [x] Restore the owner to `Active` after the last borrow ends.
- [x] Reject mutation while an owner is frozen.
- [x] Reject moving a frozen owner.
- [x] Reject dropping a frozen owner.
- [x] Allow shared borrows to be passed to nested calls.
- [x] Reject storing borrows in longer-lived structures.
- [x] Reject returning borrows from functions.
- [x] Reject borrow escape across lexical scope boundaries.
- [x] Reject `drop` applied to an `abs` binding.
- [x] Produce diagnostics that identify every active blocking borrow.

## Calls and Roles

- [x] Validate `erg`, `abs`, and `dat` parameter compatibility.
- [x] Add a centralized source-level intrinsic registry for built-in functions.
- [x] Include statement intrinsics such as `drop` in the source-level registry.
- [x] Add the initial built-in type registry for supported `Int` and `Buffer` types.
- [x] Extend the built-in type registry with initial `Array` and `Map` entries.
- [x] Define case-sensitive, separate function and type namespaces with explicit collision rules.
- [x] Define registry status metadata for adding, deprecating, and removing built-ins.
- [x] Validate declared built-in type names before code generation.
- [x] Implement positional argument binding.
- [x] Implement named argument binding.
- [x] Reject duplicate argument bindings.
- [x] Reject unknown parameter names.
- [x] Reject invalid mixtures of positional and named arguments.
- [x] Reject ambiguous positional calls.
- [x] Validate argument ownership and borrow requirements.
- [x] Validate built-in intrinsic argument types before code generation.
- [x] Validate known argument types for user-defined verb calls.
