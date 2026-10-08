# Actus Guide Migration Map

This map assigns every top-level section of `ACTUS_CODING_AGENT_GUIDE.md` to a
handbook category and destination file. Subsections remain part of the listed
destination unless a separate destination is named.

## Current guide sections

| Source section | Destination | Treatment |
|---|---|---|
| 1. The most important rule for an agent | `agent-reference/evidence-and-scope.md` | Move the implemented/design/new classification and source-of-truth workflow. |
| 2. Language mental model | `language/overview.md` | Rewrite as the compiler pipeline and language model. |
| 3. Minimal valid program | `language/first-program.md` | Move the first program, entry contract, and basic source layout. |
| 4. Lexical rules | `language/source-files-and-layout.md`, `language/comments-and-documentation.md`, `language/types-and-literals.md` | Split identifiers, comments, documentation, whitespace, punctuation, literals, and strings by topic. |
| 5. Keywords and words | `language/overview.md`, `language/declarations.md`, `ownership/roles.md`, `compiler/ffi-and-dispatch.md` | Group vocabulary by language responsibility instead of keeping one keyword list. |
| 6. Types | `language/types-and-literals.md`, `language/generics.md`, `language/enums.md` | Split primitive, user-defined, generic, const-generic, `Option`, and `Result` material. |
| 7. Ownership roles | `ownership/overview.md`, `ownership/erg.md`, `ownership/abs.md`, `ownership/dat.md`, `ownership/ins.md`, `ownership/moves-and-reuse.md`, `ownership/cleanup-and-scope.md` | Split each role and the shared state-transition rules. |
| 8. Verb declarations and calls | `language/verbs-and-contracts.md`, `modules/public-and-private-symbols.md` | Keep declaration, visibility, method, static dispatch, and dynamic dispatch guidance together by use. |
| 9. Structs and aggregates | `language/structs.md` | Move struct declarations, construction, access, methods, and ownership behavior. |
| 10. Enums and pattern matching | `language/enums.md` | Move enum declarations, `case`, `Option`, `Result`, payloads, and ownership paths. |
| 11. Conditions, loops, and assignment | `language/expressions-and-statements.md` | Move statement and expression forms, loops, assignment, branch values, and control flow. |
| 12. Operators | `language/operators.md` | Align with the operator reference and add usage examples. |
| 13. Casts and numeric safety | `language/types-and-literals.md` | Keep casts, widths, overflow, underflow, and checked numeric behavior together. |
| 14. Constants and compile-time data | `language/constants.md` | Move constants, evaluation rules, visibility, and compile-time restrictions. |
| 15. `Buffer`, `Array`, `pack`, and `Arena` | `language/arrays-and-buffers.md`, `language/packs.md` | Split storage families and keep the shared bounded-storage model in the overview. |
| 16. Foreign functions and unsafe boundaries | `compiler/ffi-and-dispatch.md`, `compiler/compiler-boundaries.md` | Separate source usage from compiler and ABI boundary rules. |
| 17. Modules, imports, and facades | `modules/overview.md`, `modules/files-and-directories.md`, `modules/canonical-facades.md`, `modules/public-and-private-symbols.md`, `modules/imports.md`, `modules/nested-modules.md` | Split the module system into reader-focused topics. |
| 18. Roles and performance implementations | `ownership/roles.md`, `language/generics.md` | Explain role contracts and `perform` implementations where each is used. |
| 19. Metadata | `language/declarations.md`, `compiler/targets-and-profiles.md`, `workflow/benchmarks.md` | Split tests, target selection, and source-limit metadata by purpose. |
| 20. Standard library | `standard-library/overview.md`, `standard-library/io.md`, `standard-library/filesystem.md`, `standard-library/paths.md`, `standard-library/strings-and-utf8.md`, `standard-library/time.md`, `standard-library/regions.md`, `standard-library/wire.md`, `standard-library/errors-and-results.md` | Create one usage page per public standard-library family. |
| 21. Build manifests and runtime profiles | `compiler/actus-toml.md`, `compiler/lockfiles.md`, `compiler/targets-and-profiles.md`, `standard-library/runtime-profiles.md` | Separate package configuration, lockfiles, targets, and runtime selection. |
| 22. CLI workflow for an agent | `workflow/project-setup.md`, `workflow/check-test-and-build.md`, `workflow/formatting.md`, `workflow/examples.md`, `workflow/debugging.md`, `workflow/contribution-workflow.md` | Rewrite as shared human and agent workflows, with agent-specific steps linked separately. |
| Current systems-language capability status | `compiler/overview.md`, `compiler/compiler-boundaries.md`, `agent-reference/evidence-and-scope.md` | Convert status-heavy material into usage boundaries and implementation references. |
| LSP and editor behavior | `compiler/diagnostics.md`, `workflow/debugging.md`, `agent-reference/diagnostic-decision-tree.md` | Explain editor behavior for users and diagnostic investigation for agents. |
| Formatter rules | `language/formatting.md`, `workflow/formatting.md` | Separate language formatting behavior from commands and workflow. |
| Diagnostics and error design | `compiler/diagnostics.md`, `agent-reference/diagnostic-decision-tree.md` | Provide user-facing error reading and agent-facing investigation paths. |
| Native and embedded-oriented design rules | `compiler/native-builds.md`, `compiler/hosted-and-freestanding.md`, `language/packs.md`, `standard-library/runtime-profiles.md` | Split language constructs, native output, and runtime/target boundaries. |
| What is not currently safe to assume | `agent-reference/evidence-and-scope.md`, `compiler/compiler-boundaries.md` | Keep unavailable or design-only features as explicit usage boundaries. |
| Recommended coding patterns | `language/verbs-and-contracts.md`, `ownership/moves-and-reuse.md`, `standard-library/errors-and-results.md` | Place each pattern beside the language or API it demonstrates. |
| Agent workflow checklist | `agent-reference/reading-order.md`, `agent-reference/source-change-checklist.md`, `agent-reference/repository-boundaries.md` | Preserve the checklist as a separate agent reference. |
| Reference map | `docs/guide/README.md` and category indexes | Replace absolute path references with handbook links and repository-relative references. |

## Existing supporting documents

These documents are source material or cross-references for the handbook:

- `docs/language/alpha-user-guide.md`
- `docs/language/alpha-application-workflow.md`
- `docs/language/alpha-guarantees.md`
- `docs/language/diagnostics.md`
- `docs/language/lexical-map.md`
- `docs/language/lsp-protocol.md`
- `docs/language/operators.md`
- `docs/language/style-and-conventions.md`
- `docs/architecture/compiler-pipeline.md`
- `docs/conformance/limitless-policy.md`
- `docs/decisions/ADR-0001-native-resource-abi.md` through the current accepted
  language, runtime, region, and Wire ADRs
- `examples/`
- `library/std/src/`
- `tests/`

The supporting documents remain separate references until the handbook files
are written and linked. Their content is not removed by this phase.

## Editorial treatment list

### Sections to split

- Lexical rules combine identifiers, comments, documentation, literals, and
  strings.
- Types combine primitive, user-defined, generic, const-generic, and result
  types.
- Ownership combines four roles, state transitions, and cleanup.
- Modules combine file layout, facades, imports, visibility, and configuration.
- The standard library combines unrelated API families.
- The CLI workflow combines project creation, validation, testing, formatting,
  and repository development.
- Capability status combines language usage, compiler boundaries, and evidence.

### Sections to cross-link

- Ownership roles link to verbs, structs, enums, arrays, buffers, modules,
  standard-library APIs, and diagnostics.
- Types and literals link to operators, casts, constants, packs, arrays, and
  generics.
- Modules and facades link to standard-library imports, public APIs, and module
  diagnostics.
- Compiler targets link to manifests, runtime profiles, native builds, and
  freestanding boundaries.
- Diagnostics link to ownership, modules, types, native builds, LSP behavior,
  and the diagnostic decision tree.

### Sections requiring current-source review

- The capability-status material contains implementation descriptions and
  evidence references that must be matched to current compiler tests.
- Standard-library entries must be compared with current public facades and
  sibling modules.
- Compiler commands must be compared with current CLI help and contribution
  instructions.
- Wire material must match ADR-0078 and the current `library/std/src/wire/`
  facade tree.
- Runtime-region material must match the current region ADRs and public API.
- Absolute repository paths must become repository-relative links in the
  handbook.

## Inventory completion checklist

- [x] Complete source-guide heading inventory recorded.
- [x] Every source section assigned to a handbook destination.
- [x] Existing language, architecture, conformance, example, library, and test
      references listed.
- [x] Sections requiring splitting identified.
- [x] Cross-link groups identified.
- [x] Sections requiring implementation-source review identified.
- [x] Destination handbook files created.
- [x] Migrated sections reviewed against this map.
