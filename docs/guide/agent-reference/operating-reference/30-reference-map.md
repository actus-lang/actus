# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 30. Reference map

Use these repository documents as focused references. The canonical Actus
repository root is:

```text
/home/magradze/Projects/actus_project/actus
```

The paths below are absolute on the development machine so an agent launched
from another project directory can locate the source of truth directly:

- `/home/magradze/Projects/actus_project/actus/README.md`: project status, build basics, runtime selection, and ABI limits.
- `/home/magradze/Projects/actus_project/actus/docs/language/alpha-user-guide.md`: implemented Alpha workflow.
- `/home/magradze/Projects/actus_project/actus/docs/language/lexical-map.md`: keywords and future/reserved vocabulary.
- `/home/magradze/Projects/actus_project/actus/docs/language/operators.md`: operator precedence and operand contracts.
- `/home/magradze/Projects/actus_project/actus/docs/language/alpha-guarantees.md`: ownership, borrowing, cleanup, and
  bounded storage guarantees.
- `/home/magradze/Projects/actus_project/actus/docs/language/style-and-conventions.md`: source style and naming.
- `/home/magradze/Projects/actus_project/actus/docs/language/lsp-protocol.md`: editor/LSP contract.
- `/home/magradze/Projects/actus_project/actus/docs/conformance/limitless-policy.md`: source-limit exceptions.
- `/home/magradze/Projects/actus_project/actus/docs/architecture/compiler-pipeline.md`: compiler architecture.
- `/home/magradze/Projects/actus_project/actus/docs/decisions/ADR-0052-core-control-flow-constants-and-type-directed-ergonomics.md`:
  current ergonomic/core direction.
- `/home/magradze/Projects/actus_project/actus/docs/decisions/ADR-0053-production-language-capability-and-wire-readiness.md`:
  production capability boundary.
- `/home/magradze/Projects/actus_project/actus/examples/`: executable language examples.
- `/home/magradze/Projects/actus_project/actus/library/std/src/`: public standard-library facades and sibling modules.
- `/home/magradze/Projects/actus_project/actus/tests/`: compiler, native, runtime, LSP, and standard-library evidence.

The source code and tests are authoritative when a prose document is stale.
If this guide and the compiler disagree, do not silently choose one: report
the inconsistency, update the guide and focused documentation, and add or
repair the test that defines the intended behavior.
