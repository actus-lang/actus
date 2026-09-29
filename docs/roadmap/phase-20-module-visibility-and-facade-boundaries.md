# Phase 20: Module Visibility and Facade-Bounded Compilation

This phase implements [ADR-0047](../decisions/ADR-0047-module-internal-scope-and-facade-visibility.md).
It gives Actus modules a real separation between their internal compilation
scope and their public facade interface.

Phase 20 addresses a compiler architecture limitation: a module's private
helpers and runtime bridges must remain available while compiling public
wrappers, but must not become visible to importing Actus programs. The phase
must solve that boundary in the compiler, semantic analyzer, code generator,
standard library, and language tooling.

## Sequencing and completion rules

- Gates are completed in order from 20.0 through 20.6.
- Every semantic visibility rule requires accepted and rejected programs.
- Standard-library changes require typed API tests and native execution tests.
- Tests belong in `tests/`; implementation files must contain no test logic.
- A passing parse or type check alone does not close a gate. The relevant
  import, semantic, code-generation, runtime, or tooling boundary must be
  exercised directly.
- No raw C-ABI bridge becomes public as a temporary compatibility measure.
- Phase 20 must not introduce Rust-style module conventions into Actus source.

## Gate 20.0: Baseline and module contract inventory

- [ ] Record the current facade, sibling discovery, export, and import flow.
- [ ] Map the current flattened `Program` representation and its limitations.
- [ ] Inventory all modules whose public declarations depend on private
      helpers, raw bridges, or private types.
- [ ] Record the existing module diagnostic range and reserve a stable
      visibility diagnostic without renderer-specific behavior.
- [ ] Add baseline positive and negative fixtures for public and private
      declarations.
- [ ] Define module identity, source provenance, cache keys, and repeated
      import behavior.
- [ ] Document the baseline in this roadmap and ADR-0047 before implementation.

## Gate 20.1: Module compilation-unit representation

- [ ] Add an explicit module-unit abstraction for facade and opened siblings.
- [ ] Preserve all internal declarations required for module analysis.
- [ ] Preserve declaration source paths, spans, sibling identity, and facade
      provenance.
- [ ] Store a separate public export table instead of using one flattened
      program as both implementation and interface.
- [ ] Reject duplicate declarations deterministically within one module unit.
- [ ] Add unit tests for module-unit construction, ordering, provenance, and
      repeated imports.

## Gate 20.2: Internal semantic scope

- [ ] Analyze facade and opened siblings in one internal module scope.
- [ ] Resolve private helpers from public wrapper bodies.
- [ ] Resolve unsafe runtime bridges only inside their declaring module.
- [ ] Preserve ownership roles, generic contracts, cleanup, and result types
      across internal references.
- [ ] Reject unresolved internal declarations before code generation.
- [ ] Add positive tests for private helper calls and typed wrappers that use
      private bridges.
- [ ] Add negative tests for duplicate, cyclic, or provenance-conflicting
      internal declarations.

## Gate 20.3: Public interface and visibility diagnostics

- [ ] Build an importer-facing interface from explicit facade exports only.
- [ ] Keep closed siblings and closed declarations out of external lookup.
- [ ] Keep raw `unsafe extern "C"` bridges private unless a future ADR
      explicitly defines a public interop contract.
- [ ] Reject direct external calls to private helpers and raw bridges.
- [ ] Reject private types, constants, and implementation details leaking into
      public signatures.
- [ ] Add a stable visibility diagnostic containing symbol, module, and
      facade guidance.
- [ ] Verify caller-side type, ownership, generic, and error contracts use the
      public interface without importing private bodies.

## Gate 20.4: Code generation and implementation emission

- [ ] Generate public wrappers from their validated module implementation
      unit, including required private dependencies.
- [ ] Emit each module implementation once when multiple callers import it.
- [ ] Preserve C ABI declarations and typed status translation at the runtime
      boundary.
- [ ] Prevent private Actus declarations from appearing in caller namespaces
      or public generated interfaces.
- [ ] Add native tests proving a public wrapper can call a private bridge.
- [ ] Add negative tests proving private bridge calls fail before linking.
- [ ] Verify deterministic object output and symbol ownership across repeated
      compilation.

## Gate 20.5: Standard-library migration

- [ ] Migrate `std::io` to the internal/public module-unit model.
- [ ] Migrate `std::fs` to the internal/public module-unit model.
- [ ] Migrate `std::path` to the internal/public module-unit model.
- [ ] Keep public APIs typed with `Result`, `Option`, ownership roles, and
      documented Actus contracts.
- [ ] Keep raw C/POSIX/Windows statuses and unsafe bridge declarations
      inaccessible to normal library consumers.
- [ ] Add positive application tests for every migrated public wrapper family.
- [ ] Add negative application tests for direct raw-bridge access and private
      sibling access.
- [ ] Update standard-library block documentation and facade exports.

## Gate 20.6: Tooling, compatibility, and closure

- [ ] Update LSP completion, hover, definition, semantic tokens, and
      diagnostics to use internal scope only inside a module and public
      interface outside it.
- [ ] Rebuild module interfaces correctly for unsaved overlays.
- [ ] Add CLI, native, semantic, and documentation regression coverage.
- [ ] Verify no private bridge is exposed through generated API metadata.
- [ ] Update README, language reference, standard-library documentation,
      ADRs, and roadmap claims consistently.
- [ ] Run formatting, compilation, Clippy, full tests, source limits, and
      diff checks.
- [ ] Record Linux, macOS, and Windows CI evidence where the affected
      compiler or standard-library paths are exercised.
- [ ] Mark ADR-0047 implemented only after all prior gates are green.

## Required acceptance matrix

| Boundary | Positive evidence | Negative evidence |
| --- | --- | --- |
| Internal sibling scope | Wrapper resolves private helper | Caller cannot resolve helper |
| Runtime bridge | Typed wrapper executes natively | Raw bridge call fails semantically |
| Facade export | Explicit `open` declaration imports | Closed sibling stays hidden |
| Public signature | Exported type/role/result remains valid | Private type leak is rejected |
| Code generation | One implementation emits for many callers | Duplicate or hidden symbol failure |
| LSP | Internal and external views differ correctly | Private symbol absent externally |
| Overlay analysis | Unsaved module changes refresh interface | Stale private export is rejected |

## Exit criteria

Phase 20 is complete only when module implementation and module interface are
distinct compiler concepts, all standard-library wrappers compile through the
internal scope, private declarations remain inaccessible externally, and the
same behavior is proven by semantic, native, CLI, and tooling tests.
