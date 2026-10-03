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

## Hierarchical module units

Module resolution produces one deterministic compilation unit for a canonical
parent facade and its explicitly opened child facades. Each directory module
has a facade whose filename matches the directory. Child implementation files
remain in the child facade's internal scope; external resolution receives only
the export table assembled by the parent facade chain. Direct child imports
are rejected before semantic analysis.

The resolver, parser aggregation, semantic visibility checks, native object
planning, formatter, LSP, and test runner consume this same module-unit
identity. Filesystem order is not an API contract, and a child directory must
not create a second object owner when its parent module is compiled.
