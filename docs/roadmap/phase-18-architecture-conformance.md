# Phase 18 Module and Architecture Conformance Report

This report is the evidence index for Gate 18.6. The resolver, module
aggregator, static architecture checks, and module fixtures enforce the
facade and compiler-boundary rules together.

## Gate evidence matrix

| Rule | Enforcement | Evidence |
| --- | --- | --- |
| Canonical directory facades | Resolver requires `<module>/<module>.act`; the repository check scans every standard-library directory | `tests/modules/resolver.rs`, `tests/architecture_conformance.rs` |
| No external access to private siblings | Export collection includes only declarations marked `open`; imported closed symbols fail semantic validation | `tests/modules/facade.rs`, `tests/modules/imports.rs` |
| No direct sibling imports | Resolver rejects a nested module path that bypasses a parent facade with `E1108` | `tests/modules/resolver.rs::rejects_direct_imports_that_bypass_a_parent_facade` |
| Deterministic sibling declarations | Siblings are sorted before parsing; duplicate declarations and duplicate performances include both source locations | `tests/modules/resolver.rs`, `tests/modules/aggregation.rs` |
| Extensionless `open` exports | Facade validation accepts only discovered sibling stems and exports only `open` declarations | `tests/modules/facade.rs` |
| Reverse pipeline dependencies | Static boundary checks reject forbidden imports and responsibility markers in lexer, parser, AST, semantic, and codegen layers | `scripts/check_architecture.sh`, `tests/architecture_conformance.rs` |
| Actus generic filenames | Static checks reject `utils.act`, `helpers.act`, `common.act`, and `misc.act` | `scripts/check_architecture.sh`, `tests/architecture_conformance.rs` |
| Rust generic filenames | The same check rejects the four generic `.rs` names under compiler sources | `scripts/check_architecture.sh`, `tests/architecture_conformance.rs` |
| Facade, sibling, visibility, and dependency fixtures | Focused module suite plus architecture suite cover accepted and rejected boundaries | `tests/modules.rs`, `tests/architecture_conformance.rs` |

## Boundary contract

The module resolver owns path validity, canonical facade discovery, root
ambiguity, sibling discovery, and direct-import rejection. The module
aggregator owns parsing, duplicate identity checks, facade export collection,
and import expansion. The semantic analyzer receives the resulting program;
it does not discover sibling files or bypass visibility boundaries.

Performances are exported through the same facade boundary as other public
declarations: `open perform` is required for external import visibility. Within
the module itself, sibling performances remain available to semantic analysis.
Their owning role, target, and method contract are still validated, and
duplicate role/target identities are rejected before code generation.

The static architecture check is run by the pre-commit hook and can be run
directly with:

```sh
scripts/check_architecture.sh
```
