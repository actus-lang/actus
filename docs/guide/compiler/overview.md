# Compiler overview

The Actus compiler turns package source into checked source, native objects, or
executables through one ordered pipeline:

```text
lexer -> parser -> AST -> semantic analysis -> ownership and cleanup
       -> native lowering -> object/linker -> executable or object artifact
```

Use the compiler to validate the language contract before asking it to emit
native output. A successful parse is not a successful type or ownership check,
and a successful check is not an executable runtime result.

The compiler reports structured diagnostics with stable codes and source spans.
The formatter, LSP, native backend, and tests use the same source contract.
