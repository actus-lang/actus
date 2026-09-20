# Actus Roadmap

Actus is developed in small, verifiable stages. Each item should be checked only after the implementation, tests, and documentation are complete.

## Phase 1: Repository and Project Discipline

- [ ] Protect the `main` branch from direct pushes.
- [ ] Require pull requests for changes to `main`.
- [ ] Require successful CI checks before merging.
- [ ] Require at least one code review for pull requests.
- [ ] Add a pull request template.
- [ ] Define and enforce the Conventional Commits format.
- [ ] Add a commit-message checker.
- [ ] Add local `pre-commit` checks.
- [ ] Add local `pre-push` checks.
- [ ] Add a pinned Rust toolchain with `rust-toolchain.toml`.
- [ ] Configure `rustfmt` and enforce formatting in CI.
- [ ] Configure Clippy and treat warnings as errors in CI.
- [ ] Run `cargo check`, `cargo test`, and `cargo clippy` in GitHub Actions.
- [ ] Add dependency auditing to CI.
- [ ] Add dependency and license policy checks.
- [ ] Add secret scanning to the repository workflow.

## Phase 2: Language Foundation

- [ ] Define the initial token set.
- [ ] Implement the lexer for Actus source files.
- [ ] Support braces, semicolons, identifiers, literals, comments, and operators.
- [ ] Add the `verb`, `erg`, `abs`, `dat`, `ref`, and `drop` keywords.
- [ ] Track source spans for every token.
- [ ] Produce structured lexer diagnostics.
- [ ] Reject unknown characters and malformed literals.
- [ ] Reject unterminated strings and comments.
- [ ] Add lexer unit tests.
- [ ] Add valid lexer fixtures.
- [ ] Add invalid lexer fixtures.

## Phase 3: Parser and AST

- [ ] Define the initial Actus grammar.
- [ ] Define the AST for programs, verbs, parameters, roles, types, blocks, statements, and expressions.
- [ ] Implement parsing for top-level declarations.
- [ ] Implement parsing for verb declarations and parameters.
- [ ] Implement parsing for blocks and nested scopes.
- [ ] Implement parsing for variable declarations.
- [ ] Implement parsing for borrow declarations using `ref`.
- [ ] Implement parsing for assignments and calls.
- [ ] Implement parsing for returns.
- [ ] Implement parsing for intrinsic `drop` statements.
- [ ] Track source spans for AST nodes.
- [ ] Produce stable parser diagnostics with error codes.
- [ ] Reject missing semicolons.
- [ ] Reject unclosed blocks and unmatched braces.
- [ ] Reject invalid roles and malformed declarations.
- [ ] Reject invalid argument syntax.
- [ ] Add valid parser fixtures.
- [ ] Add invalid parser fixtures.
- [ ] Add AST snapshot tests.

## Phase 4: Syntax Tooling

- [ ] Add `actus parse <file>`.
- [ ] Add `actus check <file>`.
- [ ] Add deterministic diagnostic rendering.
- [ ] Define stable syntax error codes.
- [ ] Implement the first `actus fmt` command.
- [ ] Define formatting rules for blocks, parameters, calls, and expressions.
- [ ] Make formatter output deterministic.
- [ ] Verify formatter idempotence.
- [ ] Add formatter snapshot tests.
- [ ] Ensure malformed input produces diagnostics instead of panics.

## Phase 5: Semantic Analyzer

### Bindings and Scopes

- [ ] Implement lexical scope frames.
- [ ] Implement binding tables.
- [ ] Reject use of undeclared identifiers.
- [ ] Reject duplicate bindings within the same scope.
- [ ] Define and enforce shadowing rules.
- [ ] Track binding source spans.

### Ownership

- [ ] Implement `erg` owner bindings.
- [ ] Implement `dat` ownership transfer at call sites.
- [ ] Transition moved caller bindings to `Moved`.
- [ ] Reject use after move.
- [ ] Reject use after explicit drop.
- [ ] Reject double drop.
- [ ] Define ownership transfer for returned values.
- [ ] Move returned local owners out of their scope.
- [ ] Reject borrowed returns in Alpha.

### Borrowing

- [ ] Implement `BorrowRecord` with identity, owner, scope, and origin span.
- [ ] Implement `abs x = ref owner`.
- [ ] Derive `Frozen` from active borrow records.
- [ ] Restore the owner to `Active` after the last borrow ends.
- [ ] Reject mutation while an owner is frozen.
- [ ] Reject moving a frozen owner.
- [ ] Reject dropping a frozen owner.
- [ ] Allow shared borrows to be passed to nested calls.
- [ ] Reject storing borrows in longer-lived structures.
- [ ] Reject returning borrows from functions.
- [ ] Reject borrow escape across lexical scope boundaries.
- [ ] Reject `drop` applied to an `abs` binding.
- [ ] Produce diagnostics that identify every active blocking borrow.

### Calls and Roles

- [ ] Validate `erg`, `abs`, and `dat` parameter compatibility.
- [ ] Implement positional argument binding.
- [ ] Implement named argument binding.
- [ ] Reject duplicate argument bindings.
- [ ] Reject unknown parameter names.
- [ ] Reject invalid mixtures of positional and named arguments.
- [ ] Reject ambiguous positional calls.
- [ ] Validate argument ownership and borrow requirements.

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
