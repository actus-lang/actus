# Actus Roadmap

Actus is developed in small, verifiable stages. Each item should be checked only after the implementation, tests, and documentation are complete.

## Current focus

**Phase 5 is the active development phase.** Earlier phases are complete except for the explicitly deferred second-maintainer review requirement. Native backend work remains preserved as completed progress while the semantic and registry foundations are finalized.

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
- [x] Run semantic analysis from `actus check`.
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
- [x] Validate known typed initializers and assignments.

### Ownership

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

## Phase 6: Deterministic Cleanup

- [x] Implement scope cleanup planning.
- [x] Drop active owned bindings in reverse declaration order.
- [x] Skip cleanup for moved bindings.
- [x] Skip cleanup for already dropped bindings.
- [x] End borrows at their lexical scope boundary.
- [x] Implement `return` scope unwinding.
- [x] Implement `break` scope unwinding.
- [x] Implement `continue` scope unwinding.
- [x] Ensure returned owners are not dropped during unwinding.
- [x] Add LIFO destruction tests.
- [x] Add early-return cleanup tests.
- [x] Add nested-scope cleanup tests.
- [x] Add move-and-drop interaction tests.

## Phase 7: Native Backend

- [x] Define the initial native emission model.
- [x] Integrate a Rust-native backend such as Cranelift.
- [x] Emit object files or native binaries without an intermediate C representation.
- [x] Add `actus build` for writing a validated native object artifact.
- [x] Lower integer locals and arithmetic expressions to native instructions.
- [x] Define and validate the native `Int` parameter and return ABI.
- [x] Lower calls between native `Int` verbs.
- [x] Lower scalar `Int` borrow roles through the native ABI.
- [x] Lower scalar `Int` ownership transfers through the native ABI.
- [x] Lower unary negative and grouped integer expressions.
- [x] Lower nested scalar scopes and scalar `drop` statements.
- [x] Lower basic native loop CFG edges for `break` and `continue`.
- [x] Implement SSA variables and phi values for loop-carried bindings.
- [x] Validate native cleanup plan references before emission.
- [x] Reject duplicate native cleanup actions within one scope.
- [x] Associate native unwind plans with their source control-flow spans.
- [x] Create unwind plans for literal and call return expressions.
- [x] Associate cleanup plans with lexical scope spans.
- [x] Lower return, break, and continue unwind plans into the native cleanup model.
- [x] Define the initial native resource handle and destructor contract.
- [x] Define the Alpha `Buffer` layout and runtime operation contract.
- [x] Implement initial native `Buffer` allocation, append, and drop operations.
- [x] Build and link the native runtime archive for executable emission.
- [x] Lower initial `Buffer` allocation, append, and explicit drop calls.
- [x] Emit automatic `Buffer` cleanup on lexical scope and control-flow unwinding.
- [x] Emit `Buffer` ownership transfers without duplicate cleanup.
- [x] Verify deterministic native object emission for identical programs.
- [x] Add native object structural golden tests.
- [x] Emit deterministic cleanup instructions.
- [x] Emit ownership transfers without duplicate cleanup.
- [x] Emit explicit borrow scopes where required by the backend.
- [x] Reject code generation when semantic analysis fails.
- [x] Add native object and architecture-scoped machine-code golden tests.
- [x] Link generated objects in CI.
- [x] Add a minimal end-to-end Actus-to-native-binary test.
- [x] Add `actus run` for temporary native execution.
- [x] Add the initial type-aware `print` intrinsic.
- [x] Extend `print` to text literals and String bindings without adding source-level print variants.

### C Library Interoperability

- [x] Define the initial C ABI boundary independently from the native backend.
- [x] Map initial primitive, opaque-handle, calling-convention, and ownership contracts.
- [x] Define target pointer-width layout rules for initial C ABI types.
- [x] Parse, model, and import external C function declarations without generating Actus-owned C code.
- [x] Define initial C ABI return ownership and lifetime contracts.
- [x] Define ABI-safe primitive, pointer, layout, and calling-convention mappings.
- [x] Define ownership and lifetime rules at the C FFI boundary.
- [x] Support linking static and shared C libraries.
- [x] Add manifest-driven static and shared library linker inputs.
- [ ] Define a reproducible header-to-Actus binding workflow.
- [x] Mark unchecked foreign declarations with explicit `unsafe extern` boundaries.
- [x] Add C library integration fixtures and native link tests.
- [x] Add a native link smoke test for an imported C symbol.

## Phase 8: Test Infrastructure

- [x] Organize tests into lexer, parser, semantic, formatter, and code-generation suites.
- [x] Add valid `.act` fixtures.
- [x] Add invalid `.act` fixtures.
- [x] Add diagnostic snapshots.
- [x] Add stable semantic diagnostic renderer tests.
- [x] Add semantic state-transition tests.
- [x] Add ownership and borrowing regression tests.
- [x] Add golden tests for generated native output.
- [x] Make test output deterministic.
- [x] Add a command for intentionally updating snapshots.
- [x] Add coverage reporting to CI.

## Phase 9: Fuzzing and Hardening

- [x] Add a deterministic malformed-input no-panic corpus.
- [x] Add deterministic generated-input no-panic hardening checks.
- [x] Add lexer fuzzing.
- [x] Add parser fuzzing.
- [x] Fuzz malformed braces and delimiters.
- [x] Fuzz malformed literals and comments.
- [x] Verify that arbitrary input never causes a compiler panic.
- [x] Add property tests for formatter idempotence.
- [x] Add property tests for parser round-tripping where applicable.
- [x] Add compiler determinism checks.
- [x] Add performance benchmarks for lexing and parsing.
- [x] Add cross-platform CI for Linux, macOS, and Windows.

## Phase 10: Documentation and Architecture Records

- [ ] Document the Alpha language guarantees.
- [ ] Document lexical, non-escaping borrow rules.
- [ ] Document ownership and cleanup semantics.
- [ ] Document diagnostic error codes.
- [ ] Document the compiler pipeline.
- [x] Add architecture decision records for major language decisions.
- [ ] Add contributor guidelines.
- [ ] Add a code of conduct.
- [ ] Add release and versioning policy.
- [x] Add automated Conventional Commit changelog generation.
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
- [ ] Emit native struct layouts and declarations for the selected backend.
- [ ] Emit field access, initialization, assignment, and cleanup.
- [ ] Define the C ABI contract for public structs.
- [ ] Add generated native output and execution tests.

### Documentation and Compatibility

- [ ] Document the complete struct model and unsupported cases.
- [ ] Add architecture decision records for layout and field ownership.
- [ ] Add migration rules if struct semantics evolve after Alpha.
- [ ] Keep the manifesto, specification, and implementation behavior synchronized.

## Future Tooling: Arca Package Ecosystem

Arca is the planned project and package manager for Actus. It will orchestrate
the compiler, builds, tests, dependency resolution, and package publication.

### Manifest and Project Model

- [x] Define the initial `Arca.toml` manifest schema.
- [ ] Define package source roots, entry points, and project layout.
- [x] Define initial package names, versions, entry points, and Alpha edition.
- [ ] Define the `Arca.lock` dependency lockfile format.
- [ ] Define debug and release profile semantics.

### Local Workflow

- [ ] Add `arca init` and `arca new`.
- [ ] Add `arca check`.
- [ ] Add `arca build` and `arca build --release`.
- [ ] Add `arca run`.
- [ ] Add `arca test`.
- [ ] Add `arca fmt` integration.

### Dependencies and Publishing

- [ ] Define dependency resolution and version constraints.
- [ ] Define local path dependencies.
- [ ] Define registry and package index behavior.
- [ ] Define package validation and reproducible archives.
- [ ] Define package signing and trust policy.
- [ ] Add `arca publish` after the registry contract is stable.
- [ ] Add package download and cache management.

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
