# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 30. Reference map

Use these repository documents as focused references. Paths are relative to the
Actus repository root:

- `README.md`: project status, build basics, runtime selection, and ABI limits.
- `docs/language/alpha-user-guide.md`: implemented Alpha workflow.
- `docs/language/lexical-map.md`: keywords and future/reserved vocabulary.
- `docs/language/operators.md`: operator precedence and operand contracts.
- `docs/language/alpha-guarantees.md`: ownership, borrowing, cleanup, and
  bounded storage guarantees.
- `docs/language/style-and-conventions.md`: source style and naming.
- `docs/language/lsp-protocol.md`: editor/LSP contract.
- `docs/conformance/limitless-policy.md`: source-limit exceptions.
- `docs/architecture/compiler-pipeline.md`: compiler architecture.
- `docs/decisions/ADR-0052-core-control-flow-constants-and-type-directed-ergonomics.md`:
  current ergonomic/core direction.
- `docs/decisions/ADR-0053-production-language-capability-and-wire-readiness.md`:
  production capability boundary.
- `examples/`: executable language examples.
- `library/std/src/`: public standard-library facades and sibling modules.
- `tests/`: compiler, native, runtime, LSP, and standard-library evidence.

The source code and tests are authoritative when a prose document is stale.
If this guide and the compiler disagree, do not silently choose one: report
the inconsistency, update the guide and focused documentation, and add or
repair the test that defines the intended behavior.
