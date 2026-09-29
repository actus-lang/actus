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
module diagnostic range covers resolver and source-boundary failures; `E1109`
is the stable visibility diagnostic added by Gate 20.3. Module source paths and spans are currently retained
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

- [x] Analyze facade and opened siblings in one internal module scope.
- [x] Resolve private helpers from public wrapper bodies.
- [x] Resolve unsafe runtime bridges only inside their declaring module.
- [x] Preserve ownership roles, generic contracts, cleanup, and result types
      across internal references.
- [x] Reject unresolved internal declarations before code generation.
- [x] Add positive tests for private helper calls and typed wrappers that use
      private bridges.
- [x] Add negative tests for duplicate and provenance-conflicting internal
      declarations.
- [deferred] Reject cyclic import graphs as part of importer-facing interface
      resolution in Gate 20.3; sibling internal scope does not recursively
      import modules and therefore has no internal sibling cycle to resolve.

### Gate 20.2 evidence

`analyze_module` now loads a `ModuleUnit` and analyzes its complete
implementation program directly. The public export table is not flattened into
that internal semantic scope. This makes private helper calls and private
unsafe C-ABI bridge calls resolvable inside the module while unresolved private
dependencies fail before code generation. Existing duplicate declaration
validation remains the provenance-conflict guard, and the module test suite
covers both accepted and rejected paths. Cyclic importer graphs are explicitly
tracked for Gate 20.3 rather than being hidden inside internal scope logic.

## Gate 20.3: Public interface and visibility diagnostics

- [x] Build an importer-facing interface from explicit facade exports only.
- [x] Keep closed siblings and closed declarations out of external lookup.
- [x] Keep raw `unsafe extern "C"` bridges private unless a future ADR
      explicitly defines a public interop contract.
- [x] Reject direct external calls to private helpers and raw bridges.
- [x] Reject private types, roles, generic bounds, and implementation details
      leaking into public signatures; constants are not a declaration category
      in the current Actus AST.
- [x] Add a stable visibility diagnostic containing symbol, module, and
      facade guidance.
- [x] Verify caller-side type, ownership, generic, and error contracts use the
      public interface without importing private bodies.

### Gate 20.3 progress evidence

The importer now validates references against the module's explicit export
table before semantic analysis. Direct calls to closed helpers and raw
`unsafe extern "C"` bridges, as well as construction of closed struct types,
produce `E1109` with the symbol, module, and canonical facade in the diagnostic
message. Public wrappers continue to be analyzed from the module's complete
internal implementation unit. The compilation-unit loader also validates every
exported verb, external verb, struct, pack, enum, role, and performance
signature, including nested result types and generic role bounds. A private
signature dependency therefore fails at the module boundary rather than being
reported later as an importer-side unknown type.

## Gate 20.4: Code generation and implementation emission

- [x] Introduce a compilation plan that keeps the caller's public program
      separate from each imported module's internal implementation unit.
- [x] Add a linker boundary that accepts one deterministic object input per
      compiled module unit while preserving the existing single-object API.
- [x] Reject unresolved cross-module native symbol collisions before codegen;
      deterministic symbol namespacing remains the follow-up implementation.
- [x] Generate public wrappers from their validated module implementation
      unit, including required private dependencies.
- [ ] Emit each module implementation once when multiple callers import it.
- [ ] Preserve C ABI declarations and typed status translation at the runtime
      boundary.
- [x] Prevent private Actus declarations from appearing in caller namespaces
      or public generated interfaces.
- [x] Add native tests proving a public wrapper can call a private implementation.
- [x] Add negative tests proving private bridge calls fail before linking.
- [ ] Verify deterministic object output and symbol ownership across repeated
      compilation.

### Gate 20.4 implementation sequence

The remaining work is intentionally ordered as one complete ABI migration.
No individual symbol family or object-emission shortcut may be marked complete
while another family still uses an unqualified name.

#### 20.4.1: Canonical module identity

- [x] Define one canonical namespace key from the resolved module identity.
- [x] Normalize module segments and derive a stable length-delimited prefix.
- [x] Define the root-program namespace and the reserved runtime namespace.
- [x] Reject empty, ambiguous, or unstable namespace components.
- [x] Add unit tests for root, nested, invalid, and separator-boundary cases.

#### 20.4.2: Symbol-identity model

- [x] Add a codegen-only symbol identity type carrying namespace, declaration
      kind, source name, and generic specialization identity.
- [x] Define escaping for identifiers and separators without collisions.
- [x] Define stable identities for verbs, external bridges, structs, packs,
      enums, roles, performances, vtables, layouts, and generated data.
- [x] Make symbol construction pure and independent of declaration order.
- [x] Add identity-based duplicate validation through a deterministic registry.
- [x] Add accepted and rejected identity tests across declaration families and
      generic specializations.

#### 20.4.3: Frontend-to-codegen symbol propagation

- [ ] Attach owning module identity to each internal implementation unit.
- [ ] Pass symbol context through declaration collection and function maps.
- [ ] Namespace direct calls, recursive calls, generic monomorphizations, and
      method/performance dispatch references.
- [ ] Namespace struct layouts, pack layouts, enum payload helpers, vtables,
      string/data globals, and cleanup/drop helpers.
- [ ] Keep public entry symbols and documented C ABI bridge names explicitly
      unqualified where their ABI contract requires it.
- [ ] Add codegen assertions that every internal reference resolves through the
      same symbol identity constructor.

#### 20.4.4: Independent module object emission

- [ ] Emit one object per `ModuleUnit`, in canonical module order.
- [ ] Emit the root caller object separately with only the public signature
      surface and the selected entry point.
- [ ] Ensure each imported implementation unit is emitted exactly once even
      when reached through repeated or transitive imports.
- [ ] Keep private implementation declarations out of caller objects and
      caller semantic namespaces.
- [ ] Record object ownership and symbol manifests before linking.
- [ ] Reject duplicate object ownership before invoking the linker.

#### 20.4.5: Linker and ABI integration

- [ ] Link the ordered object manifest without relying on filesystem order.
- [ ] Preserve runtime C ABI declarations and typed `Result` translation.
- [ ] Validate that internal namespaced symbols never replace public bridge
      symbols or the configured executable entry symbol.
- [ ] Verify hosted and freestanding entry contracts with multi-object builds.
- [ ] Add native tests for public wrappers, private helpers, external bridges,
      generic dispatch, and drop/cleanup references across objects.
- [ ] Add negative tests for private symbol leakage and unresolved references.

#### 20.4.6: Reproducibility and ownership evidence

- [ ] Generate a deterministic symbol manifest for every object build.
- [ ] Compare repeated builds byte-for-byte or document the permitted object
      metadata variance with a stable normalized comparison.
- [ ] Verify deterministic object ordering, namespace identities, and linker
      inputs across repeated and transitive-import builds.
- [ ] Verify that one module imported by multiple callers is emitted once.
- [ ] Add regression coverage for same-named declarations in separate modules.
- [ ] Mark Gate 20.4 complete only after all preceding sub-gates and the full
      quality matrix pass.

### Gate 20.4 progress evidence

`ModuleCompilationPlan` now separates the public caller program from the
internal `ModuleUnit` list used for implementation emission. Repeated imports
are deduplicated by module path, transitive module imports are collected, and
each retained unit still contains all private declarations required by its
public wrappers. The build pipeline validates the public caller scope and
compiles a separate implementation program assembled from caller-local
declarations plus the internal units. Native application tests prove that
`std::fs` wrappers execute with their private dependencies while those
declarations remain absent from caller lookup. The linker also accepts an
ordered object list and rejects an empty link input. The plan rejects
cross-module native symbol collisions with `E1110` before codegen. Per-module
object emission and deterministic symbol namespacing remain open rather than
allowing ambiguous ABI symbols to reach the linker. Native application tests
now prove that a public module wrapper can execute a private implementation,
while a direct private bridge call fails with `E1109` before code generation.
Imported public verbs are represented in the caller scope by signature-only
declarations; their bodies are analyzed and emitted only from the owning
module implementation unit. Gate 20.4.1 now provides `ModuleNamespace` with a
reserved root namespace, canonical module path, and collision-safe symbol
prefix; later codegen stages must use this identity instead of reconstructing
module names locally. Gate 20.4.2 now provides a codegen-only
`SymbolIdentity`/`SymbolRegistry` model with declaration-family tags,
collision-free escaping, generic specialization keys, and declaration-order
independent duplicate validation. The model is not yet wired into emission;
that is the explicit scope of Gate 20.4.3.

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
