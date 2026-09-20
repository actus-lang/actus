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
