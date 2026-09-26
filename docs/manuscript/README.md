# The Actus Manuscript

The Actus Manuscript is the official language guide and reference for Actus.
It combines an approachable tutorial with an executable language
specification. The compiler, parser, AST, semantic analyzer, ADRs, and tests
are the sources of truth for every normative statement.

## Documentation model

The Manuscript is maintained in English as its single canonical edition. This
keeps terminology, normative wording, examples, and compiler behavior aligned
with one authoritative source.

Each chapter follows this practical sequence:

1. Problem — the systems-programming problem being addressed;
2. Actus principle — the language rule and its rationale;
3. Code example — a minimal, valid Actus program;
4. Compiler and memory result — diagnostics, ownership transitions, layout,
   cleanup, or generated-code consequences.

Examples that claim to be valid must eventually be covered by compiler tests.
Examples marked `Planned` or `Provisional` must not be presented as stable
Actus syntax.

## Stability labels

- **Stable** — implemented, tested, and suitable for public use;
- **Implemented** — available in the compiler, but still subject to broader
  documentation or compatibility review;
- **Provisional** — an accepted design with an implementation or compatibility
  boundary that may still change;
- **Planned** — future work, not part of the current language contract.

## Chapter roadmap

- [ ] [Chapter 01: Foundations and Mental Model](en/01-foundations-and-mental-model.md)
  — Ergative linguistics, systems programming, C/Rust pain points, Actus's
  response, and the `no_std` direction.
- [ ] [Chapter 02: First Program and CLI Workflow](en/02-first-program-and-cli-workflow.md)
  — Installation, project layout, `verb main() -> Int`, compilation, and
  execution with Actus.
- [ ] [Chapter 03: Lexicon and Syntax Fundamentals](en/03-lexicon-and-syntax-fundamentals.md)
  — Keywords, bindings, verbs, parameters, roles, and types.
- [ ] [Chapter 04: Types, Structs, Enums, Option, and Result](en/04-types-structs-enums-option-result.md)
  — Primitive types, `Buffer`, data modeling, tagged unions, and null-free
  results.
- [ ] [Chapter 05: The Ergative Core](en/05-ergative-core.md)
  — `erg`, `abs`, `dat`, ownership states, linear transfer, and deterministic
  cleanup.
- [ ] [Chapter 06: Pattern Matching and Resource Cleanup](en/06-pattern-matching-and-resource-cleanup.md)
  — `case abs`, `case dat`, partial moves, branch joins, and cleanup.
- [ ] [Chapter 07: Roles, Generics, and Perform](en/07-roles-generics-and-perform.md)
  — Role contracts, generic bounds, static performance, and dispatch.
- [ ] [Chapter 08: Modules, Facades, and Imports](en/08-modules-facades-and-imports.md)
  — Directory modules, facade boundaries, sibling scope, and `import`.
- [ ] [Chapter 09: The Actus Ecosystem and Workflows](en/09-actus-ecosystem-and-workflows.md)
  — `Actus.toml`, dependencies, profiles, lockfiles, and testing.
- [ ] [Chapter 10: Unsafe, `extern "C"`, and FFI](en/10-unsafe-extern-c-and-ffi.md)
  — C ABI declarations, unsafe boundaries, ABI compatibility, and linking.
- [ ] [Chapter 11: Tooling, LSP, and Editors](en/11-tooling-lsp-and-editors.md)
  — Tree-sitter, `actus lsp`, diagnostics, navigation, and the VS Code
  extension.
- [ ] [Chapter 12: Runtime and Core Primitives](en/12-runtime-and-core-primitives.md)
  — Allocation, stack and heap behavior, intrinsics, and the `no_std`
  boundary.
- [ ] [Chapter 13: Compiler Architecture and Codegen](en/13-compiler-architecture-and-codegen.md)
  — Lexer-to-AST-to-semantic flow, SSA-oriented lowering, Cranelift, and
  self-hosting.
- [ ] [Chapter 14: Embedded Systems and Edge AI in Practice](en/14-embedded-and-edge-ai-in-practice.md)
  — Bare-metal targets, bounded memory, robotics, and edge inference.

The first implementation target is Chapter 01. Later chapters are written and
reviewed one at a time before the next chapter is started.

## Canonical references

- Language decisions: [`docs/decisions/`](../decisions/)
- Implementation roadmap: [`docs/roadmap/`](../roadmap/)
- Compiler source: `src/lexer/`, `src/parser/`, `src/ast/`, `src/semantic/`,
  and `src/codegen/`
- Executable behavior: `tests/` and the compiler quality gates in `AGENTS.md`
