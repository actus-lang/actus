# Actus Roadmap

Actus is developed in small, verifiable stages. Each item should be checked only after the implementation, tests, and documentation are complete.

## Current focus

**Phase 5 is the active development phase.** Earlier phases are complete except for the explicitly deferred second-maintainer review requirement.

Each phase is gated: implementation progress in a later phase may be preserved, but the project must return to the earliest incomplete phase before adding new work.

## Phase 1: Repository and Project Discipline

- [x] Protect the `main` branch from direct pushes.
- [x] Require pull requests for changes to `main`.
- [x] Require successful CI checks before merging.
- [ ] Require at least one code review for pull requests. (Deferred until a second maintainer joins.)
- [x] Add a pull request template.
- [x] Define and enforce the Conventional Commits format.
- [x] Add a commit-message checker.
- [x] Add local `pre-commit` checks.
- [x] Add local `pre-push` checks.
- [x] Add a pinned Rust toolchain with `rust-toolchain.toml`.
- [x] Configure `rustfmt` and enforce formatting in CI.
- [x] Configure Clippy and treat warnings as errors in CI.
- [x] Run `cargo check`, `cargo test`, and `cargo clippy` in GitHub Actions.
- [x] Add dependency auditing to CI.
- [x] Add dependency and license policy checks.
- [x] Add secret scanning to the repository workflow.

## Phase 2: Language Foundation

- [x] Define the initial token set.
- [x] Implement the lexer for Actus source files.
- [x] Support braces, semicolons, identifiers, literals, comments, and operators.
- [x] Add the `verb`, `erg`, `abs`, `dat`, `ref`, and `drop` keywords.
- [x] Track source spans for every token.
- [x] Produce structured lexer diagnostics.
- [x] Reject unknown characters and malformed literals.
- [x] Reject unterminated strings and comments.
- [x] Add lexer unit tests.
- [x] Add valid lexer fixtures.
- [x] Add invalid lexer fixtures.

## Phase 3: Parser and AST

- [x] Define the initial Actus grammar.
- [x] Define the AST for programs, verbs, parameters, roles, types, blocks, statements, and expressions.
- [x] Implement parsing for top-level declarations.
- [x] Implement parsing for verb declarations and parameters.
- [x] Implement parsing for blocks and nested scopes.
- [x] Implement parsing for variable declarations.
- [x] Implement parsing for borrow declarations using `ref`.
- [x] Implement parsing for assignments and calls.
- [x] Implement parsing for returns.
- [x] Implement parsing for intrinsic `drop` statements.
- [x] Track source spans for AST nodes.
- [x] Produce stable parser diagnostics with error codes.
- [x] Reject missing semicolons.
- [x] Reject unclosed blocks and unmatched braces.
- [x] Reject invalid roles and malformed declarations.
- [x] Reject invalid argument syntax.
- [x] Add valid parser fixtures.
- [x] Add invalid parser fixtures.
- [x] Add AST snapshot tests.

## Phase 4: Syntax Tooling

- [x] Add `actus parse <file>`.
- [x] Add `actus check <file>`.
- [x] Add deterministic diagnostic rendering.
- [x] Define stable syntax error codes.
- [x] Implement the first `actus fmt` command.
- [x] Define formatting rules for blocks, parameters, calls, and expressions.
- [x] Make formatter output deterministic.
- [x] Verify formatter idempotence.
- [x] Add formatter snapshot tests.
- [x] Ensure malformed input produces diagnostics instead of panics.

## Phase 5: Semantic Analyzer

### Bindings and Scopes

- [x] Implement lexical scope frames.
- [x] Implement binding tables.
- [x] Reject use of undeclared identifiers.
- [x] Reject duplicate bindings within the same scope.
- [x] Define and enforce shadowing rules.
- [x] Track binding source spans.

### Ownership

- [x] Implement `erg` owner bindings.
- [x] Implement `dat` ownership transfer at call sites.
- [x] Transition moved caller bindings to `Moved`.
- [x] Reject use after move.
- [x] Reject use after explicit drop.
- [x] Reject double drop.
- [x] Define ownership transfer for returned values.
- [x] Move returned local owners out of their scope.
- [x] Reject borrowed returns in Alpha.

### Borrowing

- [x] Implement `BorrowRecord` with identity, owner, scope, and origin span.
- [x] Implement `abs x = ref owner`.
- [x] Derive `Frozen` from active borrow records.
- [x] Restore the owner to `Active` after the last borrow ends.
- [x] Reject mutation while an owner is frozen.
- [x] Reject moving a frozen owner.
- [x] Reject dropping a frozen owner.
- [x] Allow shared borrows to be passed to nested calls.
- [ ] Reject storing borrows in longer-lived structures.
- [x] Reject returning borrows from functions.
- [x] Reject borrow escape across lexical scope boundaries.
- [x] Reject `drop` applied to an `abs` binding.
- [x] Produce diagnostics that identify every active blocking borrow.

### Calls and Roles

- [x] Validate `erg`, `abs`, and `dat` parameter compatibility.
- [x] Implement positional argument binding.
- [x] Implement named argument binding.
- [x] Reject duplicate argument bindings.
- [x] Reject unknown parameter names.
- [x] Reject invalid mixtures of positional and named arguments.
- [x] Reject ambiguous positional calls.
- [x] Validate argument ownership and borrow requirements.

## Phase 6: Deterministic Cleanup

- [ ] Implement scope cleanup planning.
- [ ] Drop active owned bindings in reverse declaration order.
- [ ] Skip cleanup for moved bindings.
- [ ] Skip cleanup for already dropped bindings.
- [ ] End borrows at their lexical scope boundary.
- [ ] Implement `return` scope unwinding.
- [ ] Implement `break` scope unwinding.
- [ ] Implement `continue` scope unwinding.
- [ ] Ensure returned owners are not dropped during unwinding.
- [ ] Add LIFO destruction tests.
- [ ] Add early-return cleanup tests.
- [ ] Add nested-scope cleanup tests.
- [ ] Add move-and-drop interaction tests.

## Phase 7: C99 Backend

- [ ] Define the initial C emission model.
- [ ] Emit valid C99 for the supported Actus subset.
- [ ] Emit deterministic cleanup instructions.
- [ ] Emit ownership transfers without duplicate cleanup.
- [ ] Emit explicit borrow scopes where required by the backend.
- [ ] Reject code generation when semantic analysis fails.
- [ ] Add generated C golden tests.
- [ ] Compile generated C in CI.
- [ ] Add a minimal end-to-end Actus-to-native-binary test.

## Phase 8: Test Infrastructure

- [ ] Organize tests into lexer, parser, semantic, formatter, and code-generation suites.
- [ ] Add valid `.act` fixtures.
- [ ] Add invalid `.act` fixtures.
- [ ] Add diagnostic snapshots.
- [ ] Add semantic state-transition tests.
- [ ] Add ownership and borrowing regression tests.
- [ ] Add golden tests for generated C.
- [ ] Make test output deterministic.
- [ ] Add a command for intentionally updating snapshots.
- [ ] Add coverage reporting to CI.

## Phase 9: Fuzzing and Hardening

- [ ] Add lexer fuzzing.
- [ ] Add parser fuzzing.
- [ ] Fuzz malformed braces and delimiters.
- [ ] Fuzz malformed literals and comments.
- [ ] Verify that arbitrary input never causes a compiler panic.
- [ ] Add property tests for formatter idempotence.
- [ ] Add property tests for parser round-tripping where applicable.
- [ ] Add compiler determinism checks.
- [ ] Add performance benchmarks for lexing and parsing.
- [ ] Add cross-platform CI for Linux, macOS, and Windows.

## Phase 10: Documentation and Architecture Records

- [ ] Document the Alpha language guarantees.
- [ ] Document lexical, non-escaping borrow rules.
- [ ] Document ownership and cleanup semantics.
- [ ] Document diagnostic error codes.
- [ ] Document the compiler pipeline.
- [ ] Add architecture decision records for major language decisions.
- [ ] Add contributor guidelines.
- [ ] Add a code of conduct.
- [ ] Add release and versioning policy.
- [ ] Keep the README and manifesto synchronized with implemented behavior.

## Phase 11: Complete Struct System

Structs are intentionally deferred until the ownership, borrowing, and cleanup foundations are stable. This phase must implement structs as a complete language and compiler feature, not as parser-only syntax.

### Syntax and AST

- [ ] Define struct declaration grammar and source-span rules.
- [ ] Parse named fields with explicit types.
- [ ] Parse struct literals and field initialization.
- [ ] Parse field access and field assignment.
- [ ] Represent structs and fields in the AST with dedicated modules.
- [ ] Reject duplicate struct and field names.
- [ ] Reject unknown fields and missing required fields.

### Semantic Model

- [ ] Add a type environment for struct declarations.
- [ ] Validate field types and recursive type references.
- [ ] Define field visibility and access rules.
- [ ] Define ownership semantics for `erg`, `abs`, and `dat` fields.
- [ ] Define move semantics for whole structs and individual fields.
- [ ] Reject partial use after moving a field.
- [ ] Define and enforce struct initialization invariants.
- [ ] Define copy, move, and assignment behavior explicitly.

### Borrowing and Lifetimes

- [ ] Define whether Alpha structs may contain `abs` fields.
- [ ] Reject self-referential and escaping borrow fields unless a lifetime model exists.
- [ ] Reject storing lexical borrows in longer-lived structs.
- [ ] Validate borrow access through struct fields.
- [ ] Track field-level borrow records where supported.
- [ ] Add diagnostics identifying the struct field and blocking borrow.

### Layout and Destruction

- [ ] Define deterministic field declaration order.
- [ ] Define size, alignment, and padding rules.
- [ ] Define packed and externally represented struct policies.
- [ ] Generate field-level cleanup in reverse declaration order.
- [ ] Handle moved and dropped fields without double cleanup.
- [ ] Add layout and destruction golden tests.

### Methods and Backend

- [ ] Define struct method syntax and receiver roles.
- [ ] Validate receiver ownership and borrow behavior.
- [ ] Emit valid C99 struct declarations.
- [ ] Emit field access, initialization, assignment, and cleanup.
- [ ] Define the C ABI contract for public structs.
- [ ] Add generated C and native execution tests.

### Documentation and Compatibility

- [ ] Document the complete struct model and unsupported cases.
- [ ] Add architecture decision records for layout and field ownership.
- [ ] Add migration rules if struct semantics evolve after Alpha.
- [ ] Keep the manifesto, specification, and implementation behavior synchronized.

## Future Language Extensions

These features are intentionally outside the initial Alpha core and require a separate design decision:

- [ ] Explicitly scoped views.
- [ ] Zero-copy slices.
- [ ] Borrowed iterators.
- [ ] Borrowed return values.
- [ ] Struct fields containing references.
- [ ] Advanced lifetime relationships.
- [ ] Concurrency and thread-safety semantics.
- [ ] Self-hosting compiler stages.
