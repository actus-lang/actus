# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 2. Language mental model

Actus source is compiled through a one-way pipeline:

```text
source -> lexer -> parser -> AST -> semantic analysis -> ownership/cleanup
       -> native lowering -> object/linker -> executable or object artifact
```

The compiler owns four distinct concerns:

- syntax: tokens, declarations, expressions, statements, and spans;
- semantics: types, ownership roles, borrowing, module visibility, constant
  evaluation, and diagnostics;
- cleanup: deterministic destruction and scope unwinding;
- native behavior: ABI layout, bounds checks, calls, branches, and object code.

Do not move semantic rules into the lexer, ownership rules into the parser,
or backend-specific types into the AST. Do not make code generation repair an
invalid semantic program. Do not make semantic analysis print directly to the
terminal; return structured diagnostics instead.

The central language idea is that ownership is visible in source syntax. The
words `erg`, `abs`, `dat`, and `ins` are not comments and are not stylistic
annotations. They are part of the compiler-checked contract.
