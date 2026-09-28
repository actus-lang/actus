# Phase 18 Frontend Completeness Report

This report is the evidence index for Gate 18.4. Every declared frontend rule
has an accepted fixture and a rejected fixture. Existing focused suites remain
the detailed regression tests; `tests/frontend_completeness.rs` provides the
single cross-rule acceptance matrix.

## Rule evidence matrix

| Rule | Accepted fixture | Rejected fixture |
| --- | --- | --- |
| Unknown keywords | `parser_declaration_and_metadata_rules_have_positive_and_negative_fixtures` | Same test: `verbb` -> `E0009` |
| Unknown types | `name_resolution_rules_have_positive_and_negative_fixtures` | Same test: `Missing` -> `E1023` |
| Unknown verbs and calls | Same test: `known` call | Same test: `missing` -> `E1069` |
| Generic type keys | `malformed_generic_keys_and_primitive_rules_have_fixtures` | Same test: malformed `Box[` diagnostic -> `E1080` |
| Unknown fields | Same test: `point.x` | Same test: `point.missing` -> `E1031` |
| Modules and imports | `module_rules_have_positive_and_negative_fixtures` | Same test: missing import -> module error |
| Metadata attributes and targets | Known `target` and `test` metadata | Unknown attribute -> `E0006`; unknown target -> `E0007` |
| Metadata attachment | Known verb metadata | Metadata on `struct` -> parser rejection |
| Duplicate declarations | Distinct `Left` and `Right` declarations | Duplicate `Point`; sibling duplicates are covered by `tests/modules/aggregation.rs` |
| Ambiguous calls and module roots | Named call and one module root | Ambiguous positional call; ambiguous module roots |
| Return paths | `answer() -> Int` returns a value | Missing return value is rejected |
| Generic parameters, bounds, and applications | `Reader` bound and `Box[Int]` | Arity and constraint mismatches |
| Primitive widths, signedness, and ranges | `u1`, `i8`, and `f32` | `u1 = 2` range violation; overflow coverage is in `tests/semantic_intrinsics.rs` |

## Declaration completeness classification

The following declarations are intentionally validated before code generation
but do not lower to native executable functions:

| Declaration | Classification | Evidence |
| --- | --- | --- |
| `unsafe extern "C" verb` | Imported ABI contract; the body is supplied outside Actus | `non_codegen_declarations_are_explicitly_accepted` and `tests/ffi.rs` |
| `import` | Module visibility directive | `module_rules_have_positive_and_negative_fixtures` and `tests/modules/imports.rs` |
| Facade `open` | Public sibling export and visibility boundary | `tests/modules/facade.rs` and `tests/modules/aggregation.rs` |
| Empty `struct` | Valid zero-sized aggregate with type/layout semantics | `non_codegen_declarations_are_explicitly_accepted` |
| Marker `role` | Valid contract with no required methods | `non_codegen_declarations_are_explicitly_accepted` |
| Empty `perform` for a marker role | Explicit implementation of the marker contract | `non_codegen_declarations_are_explicitly_accepted` |

No declaration is accepted merely because it is registered. New declaration
kinds must add an AST rule, semantic validation, code-generation behavior or a
documented non-codegen classification, and both accepted and rejected fixtures.
