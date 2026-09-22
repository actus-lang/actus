# Actus Compiler Pipeline

Actus uses a one-directional compilation pipeline:

```text
source -> lexer -> parser -> AST -> semantic analyzer -> native codegen
```

- The lexer produces tokens and source spans.
- The parser builds syntax-only AST nodes and does not perform ownership
  checking.
- The AST is the frontend representation shared by later phases.
- The semantic analyzer resolves names, types, ownership, borrow records,
  receiver roles, and cleanup plans.
- The native backend emits code only after semantic analysis succeeds.

Diagnostics are data owned by the frontend and are rendered separately by the
CLI. Backend-specific types do not cross into lexer, parser, or AST modules.
Dependencies must move forward through the pipeline; reverse dependencies are
not permitted.
