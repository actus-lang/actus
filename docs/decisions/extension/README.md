# Extension Architecture Decisions

This directory contains the seven proposed, compiler-backed developer
experience capabilities planned for Phase 17, after Phase 16 standard-library
completion:

1. `ADR-0032` — Memory and Ownership Graph Inspector;
2. `ADR-0033` — Pack and MMIO Layout Viewer;
3. `ADR-0034` — Target and Board Manager;
4. `ADR-0035` — Debug Adapter Integration;
5. `ADR-0036` — CodeLens Workflows;
6. `ADR-0037` — Compiler-Backed Zero-Allocation Analysis; and
7. `ADR-0038` — Deterministic C Header Export.

The shared LSP execution and protocol foundation is defined by
`ADR-0048` — LSP Execution, Cancellation, Progress, and Bounded Responses.
It is accepted and verified by the Gate 17.0.10 response-contract snapshots,
visibility tests, and Linux/macOS/Windows CI matrix.

The VS Code Activity Bar and Sidebar are the shared presentation shell for
these capabilities, not an independent semantic subsystem. All views consume
versioned compiler/LSP contracts. The extension must not duplicate parsing,
ownership analysis, layout calculation, target selection, or ABI decisions.

The order is intentional: establish schemas and read-only inspection first,
then add workflow actions, target/debug integration, and finally compiler
backed safety/interoperability exports. Phase 16 keeps only the baseline
compiler, Tree-sitter, VS Code, and LSP parity needed to validate the current
language and standard library.
