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

- [x] Record the current facade, sibling discovery, export, and import flow.
- [x] Map the current flattened `Program` representation and its limitations.
- [x] Inventory all modules whose public declarations depend on private
      helpers, raw bridges, or private types.
- [x] Record the existing module diagnostic range and reserve a stable
      visibility diagnostic without renderer-specific behavior.
- [x] Add baseline positive and negative fixtures for public and private
      declarations.
- [x] Define module identity, source provenance, cache keys, and repeated
      import behavior.
- [x] Document the baseline in this roadmap and ADR-0047 before implementation.

### Gate 20.0 baseline evidence

The current resolver identifies a directory module through its canonical
facade and deterministically orders the facade followed by its sibling source
files. `parse_module` lexes and parses every selected source, validates
duplicate declaration identities, and returns one flattened `Program`. That
program is the current internal analysis scope, so a public wrapper can call a
private helper from an opened sibling.

The baseline inventory covers the currently boundary-sensitive standard
library modules: `std::io` has private stdout, stderr, stdin, cursor, buffered,
copy, and raw runtime bridge declarations; `std::fs` has private file,
metadata, options, and filesystem-operation bridges; and `std::path` has
private storage, parser, component, predicate, normalization, builder, and
POSIX/Windows bridge declarations. These modules are the required migration
set for Gate 20.5. Their public facade wrappers are already typed, while the
raw `unsafe extern "C"` declarations are currently present in opened sibling
sources and therefore depend on the missing internal/public scope split.

`exports_module` separately scans the facade's `open` sibling declarations and
collects only declarations explicitly marked `open` from those siblings.
`resolve_imports` uses that export table to filter declarations copied into an
importer's flattened `Program`. Repeated imports are deduplicated by module
path, but the module implementation and importer interface are not yet
represented as separate compiler objects.

The baseline fixtures in `tests/modules/baseline.rs` prove all three current
boundaries: the internal flattened program retains private declarations, a
public wrapper resolves a private helper internally, and an external caller
cannot resolve that helper through the facade. The existing `E1100`-`E1108`
module diagnostic range is reserved for the visibility diagnostic that Gate
20.3 will make explicit. Module source paths and spans are currently retained
for duplicate and parse diagnostics; cache-key and module-identity behavior
is currently the resolver's canonical module path plus deterministic source
ordering.

## Gate 20.1: Module compilation-unit representation

- [x] Add an explicit module-unit abstraction for facade and opened siblings.
- [x] Preserve all internal declarations required for module analysis.
- [x] Preserve declaration source paths, spans, sibling identity, and facade
      provenance.
- [x] Store a separate public export table instead of using one flattened
      program as both implementation and interface.
- [x] Reject duplicate declarations deterministically within one module unit.
- [x] Add unit tests for module-unit construction, ordering, provenance, and
      repeated imports.

### Gate 20.1 evidence

`ModuleUnit` now groups a canonical module identity, ordered facade and sibling
source records, the complete internal implementation `Program`, and an
independent `ModuleExports` interface. `ModuleSource` retains each source path,
facade/sibling identity, and parsed declaration program, while the existing
aggregation path continues to enforce duplicate declaration rejection before
the unit is constructed. The identity source key is derived from the
normalized, resolver-ordered source paths, so repeated loads are stable.

The implementation is exposed through `load_module_unit` without changing the
existing importer behavior. `tests/modules/compilation_unit.rs` verifies
internal declarations versus public exports, deterministic source ordering and
provenance, and repeated-load identity/export stability. Gate 20.2 will make
this unit the semantic scope boundary.

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
