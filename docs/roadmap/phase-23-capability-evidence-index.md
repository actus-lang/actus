# Phase 23 Capability Evidence Index

This index is the single navigation point for the evidence that closes Phase
23. Each row identifies the gate evidence and the executable tests that prove
the capability across the relevant compiler layers.

| Gate | Capability | Evidence | Primary executable evidence |
|---|---|---|---|
| 23.0 | Baseline and architecture contract | [baseline evidence](phase-23-gate-23.0-baseline.md) | `tests/fixtures/phase23/boolean_literal_baseline.act`, `const_generic_baseline.act`, and `nested_place_call_baseline.act`; source-limit and documentation checks |
| 23.1 | `Usize` const generics and bounded array capacities | [const-generic evidence](phase-23-gate-23.1-const-generics.md) | `tests/parser/generics.rs`, `tests/semantic/generics.rs`, `tests/codegen/layout.rs`, and native const-generic array tests |
| 23.2 | Boolean literal expressions and short-circuiting | [boolean-literal evidence](phase-23-gate-23.2-boolean-literals.md) | parser, semantic, formatter, LSP, and native Boolean literal tests |
| 23.3 | Nested places and indexed assignment | [nested-place evidence](phase-23-gate-23.3-nested-places.md) | `tests/parser/expressions.rs`, `tests/semantic/arrays.rs`, and `tests/arrays_cli.rs::executes_nested_array_struct_place_assignment` |
| 23.4 | Nested calls and statement blocks | [nested-call evidence](phase-23-gate-23.4-nested-calls.md) | nested call parser, semantic role, try-propagation, and native execution tests |
| 23.5 | Typed nested control-flow joins | [control-flow evidence](phase-23-gate-23.5-control-flow-joins.md) | nested value-producing conditionals, diverging branches, and branch cleanup tests |
| 23.6 | Formatter, LSP, and diagnostics parity | [Phase 23 roadmap Gate 23.6](phase-23-systems-language-capability-foundation.md#gate-236-formatter-lsp-and-diagnostics-parity) | `tests/formatter/`, `tests/lsp/semantic.rs`, `tests/lsp/workspace.rs`, and diagnostics suites |
| 23.7 | Strict, native, and object acceptance | [strict/native evidence](phase-23-gate-23.7-strict-native-acceptance.md) | `tests/examples_cli.rs::phase23_capability_package_passes_strict_test_native_and_object_acceptance` |
| 23.8 | Systems-readiness package contract | [readiness evidence](phase-23-gate-23.8-systems-readiness.md) | `tests/examples_cli.rs::phase23_readiness_package_passes_strict_and_native_acceptance` and `tests/lsp/semantic.rs::lsp_understands_the_phase23_readiness_source_without_overlay_drift` |

## Repository-level acceptance commands

The gate evidence is accepted only with the repository quality checks also
passing:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
scripts/check_source_limits.sh
scripts/check_documentation.sh
git diff --check
```

The implementation remains target-neutral. Phase 23 adds language and
tooling capability; target profiles, protocol libraries, and AIE-specific
lowering are separate concerns and are not hidden in these gates.
