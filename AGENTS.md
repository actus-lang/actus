# Actus Engineering Rules

This document defines the mandatory engineering rules for the Actus repository. These rules exist to keep the compiler modular, reviewable, testable, and predictable as the project grows.

## 1. Architectural Principles

Actus must be developed as a modular compiler, not as a collection of unrelated implementation details.

Every module must have one clear responsibility. Unrelated functionality must be split into separate modules before implementation grows large.

The compiler pipeline is strictly one-directional:

```text
lexer -> parser -> ast -> semantic -> codegen
```

Dependencies may move forward through this pipeline only. Reverse dependencies are strictly forbidden.

The following boundaries are mandatory:

- The lexer must not contain parser or semantic logic.
- The parser must not perform ownership or borrow checking.
- AST types must not depend on C99 code-generation details.
- The semantic analyzer must not print directly to the terminal.
- The code generator must not infer or repair semantic correctness.
- Diagnostics must be represented independently from terminal rendering.
- Backend-specific types must not leak into frontend modules.

## 2. Required Source Structure

The source tree should follow this structure as the compiler grows:

```text
src/
├── main.rs
├── cli/
│   └── mod.rs
├── lexer/
│   ├── mod.rs
│   ├── token.rs
│   └── scanner.rs
├── parser/
│   ├── mod.rs
│   ├── grammar.rs
│   └── parser.rs
├── ast/
│   ├── mod.rs
│   ├── expr.rs
│   ├── stmt.rs
│   └── decl.rs
├── diagnostics/
│   ├── mod.rs
│   ├── error.rs
│   └── renderer.rs
├── semantic/
│   ├── mod.rs
│   ├── scopes.rs
│   ├── bindings.rs
│   ├── ownership.rs
│   └── borrowing.rs
└── codegen/
    ├── mod.rs
    └── c99.rs
```

This structure is a guide for the initial implementation and may grow with new, clearly justified modules.

Module files such as `mod.rs` should define module boundaries and public exports. Substantial implementation logic belongs in responsibility-specific files.

The following generic filenames are forbidden:

- `utils.rs`
- `helpers.rs`
- `misc.rs`
- `common.rs`, unless the module has a precise, documented architectural role

Files must be named after their primary responsibility.

## 3. File Size Limits

File size limits are architectural guardrails and must be treated as mandatory review rules.

- Preferred size: 300 lines or fewer.
- Warning threshold: 400 lines.
- Hard limit: 500 lines.

A file must not exceed 500 lines without an explicit architectural exception documented in the pull request. The exception must explain why splitting the file would damage cohesion and identify a future decomposition path if one exists.

Generated files, fixture data, and intentionally tabular test data may be exempt when they are clearly marked and excluded from ordinary source modules.

## 4. Function Size Limits

- Preferred size: 30 lines or fewer.
- Warning threshold: 40 lines.
- Hard limit: 60 lines.

A function must not exceed 60 lines without an explicit architectural exception documented in the pull request.

Each function must have one clear responsibility. Large conditionals, repeated conversion logic, and independent validation stages must be extracted into named functions or dedicated modules.

A large `match` expression may be acceptable when each branch is short, cohesive, and part of one operation. It must not be used to hide multiple unrelated responsibilities.

## 5. Types and Data Structures

- Each major domain type should have one primary source file.
- AST type families should be separated by responsibility.
- Semantic state types must not be mixed with parser-only types.
- Backend types must not be introduced into the AST or lexer.
- Large `impl` blocks must be split by responsibility where practical.
- State transitions must be represented explicitly rather than encoded through undocumented flags.
- Ownership and borrow state must remain distinguishable from syntax state.

## 6. Naming Rules

- Types and traits use `PascalCase`.
- Functions, methods, variables, and modules use `snake_case`.
- Enum variants use `PascalCase`.
- Constants use `SCREAMING_SNAKE_CASE`.
- Diagnostic codes use stable `E####` identifiers.
- Names must describe domain meaning rather than implementation convenience.
- Abbreviations are forbidden unless they are established language or domain terms.
- Generic names such as `data`, `thing`, `item`, and `value` should be replaced with precise names when the context does not make the meaning obvious.

## 7. Visibility and Public API

- Use `pub` only when a symbol is part of an intentional module API.
- Keep implementation details private.
- Use `pub use` only to provide a deliberate facade.
- Every new public type, function, trait, or constant must have a clear reason to be public.
- Public API changes must include tests and documentation updates where applicable.
- Circular dependencies must be solved by changing the abstraction boundary, not by adding shortcuts.

## 8. Testing Rules

Tests must be organized by the same conceptual boundaries as production code:

```text
tests/
├── lexer/
├── parser/
├── diagnostics/
├── semantic/
├── cleanup/
└── codegen/
```

Every new module must include relevant tests. Every bug fix must include a regression test unless the change is strictly documentation or build configuration.

Tests should verify one rule or behavior at a time. Large all-in-one tests are discouraged because they make failures difficult to diagnose.

The compiler must test both accepted and rejected programs. Negative tests are essential for ownership, borrowing, syntax, diagnostics, and cleanup behavior.

Tests should cover, where applicable:

- valid and invalid tokens;
- parser acceptance and rejection;
- source spans;
- stable diagnostics;
- ownership transitions;
- borrow records and frozen owners;
- use-after-move and use-after-drop;
- non-escaping borrow rules;
- deterministic LIFO cleanup;
- early return, `break`, and `continue` unwinding;
- generated C output.

## 9. Change and Commit Rules

- One commit must represent one logical change.
- Unrelated refactoring must not be mixed into feature or bug-fix commits.
- New behavior must include or update tests.
- Public API changes must include documentation updates.
- A file exceeding the hard size limit must not be introduced without documented approval.
- Commit messages must follow the Conventional Commits format:

```text
type(scope): description
```

Allowed types are:

```text
feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert
```

Commit descriptions must be specific, concise, and written in the imperative mood where practical.

## 10. Required Quality Checks

Before a change is considered complete, the following checks must pass:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Additional checks may be required for parser, semantic analyzer, generated code, fuzzing, or release changes.

The CI system must enforce the same checks as local development. Local hooks are useful for fast feedback, but CI is the final authority.

## 11. Documentation and Decisions

Major architectural decisions must be documented before or alongside implementation. This includes changes to:

- ownership or borrow semantics;
- compiler phase boundaries;
- AST representation;
- diagnostic behavior;
- generated-code contracts;
- public command-line interfaces;
- repository or release policy.

Architecture decision records should be stored under:

```text
docs/decisions/
```

The README, manifesto, roadmap, and implementation must not contradict one another. When behavior changes, all affected documentation must be updated in the same logical change.

## 12. Review Standard

A change is ready for review only when:

- its responsibility is clear;
- its module boundaries are correct;
- file and function limits are respected;
- tests cover the behavior and relevant failure cases;
- diagnostics are deterministic;
- the required quality checks pass;
- no unrelated files or refactors are included;
- documentation reflects the implemented behavior.

When a design appears to require a large file, a large function, a generic utility module, or a reverse dependency, stop and revisit the architecture before adding code.

## Actus Language & Standard Library Coding Standards

These rules apply specifically to Actus source files and the Actus standard
library.

### 1. Actus Module Facades and Modularity

- A directory module has one canonical facade file whose name matches the
  module directory. For example, `library/std/src/io/io.act` is the facade
  for the `io` module.
- The facade is the public gateway. It re-exports sibling declarations with
  extensionless `open` declarations such as `open stdin;` and `open stdout;`.
- Sibling `.act` files share the module's internal scope and may use one
  another's declarations without import boilerplate.
- Only declarations exposed through the facade are visible to external
  consumers.
- Sibling discovery and facade exports must be deterministic and duplicate
  declarations must be rejected at compile time.
- Actus source and standard-library modules must follow Actus facade and
  sibling rules; no unrelated language's module conventions belong here.

### 2. File Size and Proactive Modularity

- Actus source files and standard-library modules must remain below the
  repository's 500-line hard limit.
- At approximately 400 lines, decomposition must be planned before further
  feature growth.
- Decompose by domain responsibility: facade, error types, ownership-aware
  input, output bridges, and stream operations belong in separate sibling
  files when they are independently understandable.
- Do not create monolithic standard-library modules merely to avoid defining
  a clear facade boundary.

### 3. C ABI and Typed Error Boundaries

- C ABI bridges must use explicit, documented status contracts. Arithmetic
  encodings such as `status - 2` or `status - 1` are forbidden.
- A byte-oriented bridge returns a non-negative actual byte count on success,
  `-1` for failure, and `-2` for end-of-stream where that status applies.
- Actus public APIs must translate raw C statuses into `Result[T, E]` before
  exposing them to application code.
- Callers should use the `?` operator for compatible error propagation.
- Ownership roles must be checked before crossing the ABI boundary, and the
  bridge must not hide allocation, ownership transfer, or cleanup behavior.

### 4. Ownership Roles and Zero-Allocation I/O

- `erg` identifies the active owner and the binding permitted to mutate its
  resource.
- `abs` identifies a read-only, non-owning view; it does not consume or drop
  the source resource.
- `ins` identifies an exclusive call-scope loan. An `ins Buffer` may be filled
  in place and is restored to the caller after the call without a new buffer
  allocation.
- `dat` identifies a terminal ownership transfer to the callee.
- Streaming input should accept caller-provided `ins Buffer` storage so loops
  can reuse the same allocation and remain zero-allocation.
- Output operations should accept `abs Buffer` when they only inspect bytes.

### 5. Public Actus Documentation

- Every public Actus type, enum, external bridge, and verb must have a `///`
  documentation comment.
- Documentation must state the relevant ownership role, return value, error
  variants, side effects, and C ABI relationship where applicable.
- `Result.Ok(count)` must be documented as the exact number of processed
  bytes, not as a success flag or encoded status.
- Comments must describe implemented behavior and must not promise hidden
  runtime guarantees that the implementation does not provide.

### 6. Actus Quality Gates

Before committing Actus compiler or standard-library changes, run the required
repository checks and any relevant native or semantic integration tests:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
scripts/check_source_limits.sh
git diff --check
```

Public API changes require documentation and tests. Ownership, borrowing, C
ABI, and standard-library changes require both accepted and rejected cases;
native runtime behavior must be verified with an execution test where the
change crosses the code-generation boundary.
