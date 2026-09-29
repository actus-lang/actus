# ADR-0047: Module-Internal Scope and Facade-Bounded Visibility

- Status: Proposed
- Date: 2026-09-29
- Scope: Actus module aggregation, semantic visibility, code generation, and
  standard-library implementation boundaries

## Context

Actus modules use a named facade as their public gateway. A directory module
contains one facade file and responsibility-specific sibling `.act` files. A
sibling declaration may be used by another declaration in the same module,
but only declarations deliberately exposed through the facade are part of the
module's external API.

This contract is especially important for the standard library. Public
wrappers such as `std::io` and `std::fs` must expose typed Actus operations,
while their implementation may depend on unsafe C-ABI bridges. The bridge is
an implementation detail: it may use raw pointers, platform statuses, or
backend-specific representations that must not become ordinary application
APIs.

The current aggregation model parses the facade and siblings into one
`Program`, computes the facade exports, and then flattens the selected
declarations into the importing program. That representation has two invalid
outcomes:

1. If private declarations are included, an importing program can call an
   internal helper or raw runtime bridge directly.
2. If private declarations are removed before semantic analysis, an exported
   wrapper can no longer resolve the private bridge used in its own body.

The failure is architectural, not a missing `open` keyword. Filtering names
after flattening cannot represent both the module's complete implementation
scope and its restricted external interface. Removing raw bridges from the
source is also not acceptable: the typed wrapper still needs a real runtime
boundary, and a facade must not become a collection of declarations that only
compiles in isolation.

The compiler therefore needs a first-class distinction between a module's
internal compilation unit and its public import interface.

## Decision

### 1. Introduce a module compilation unit boundary

Module aggregation shall produce a logical module unit with at least these
responsibilities:

- retain the facade and all selected sibling sources with source provenance;
- retain the complete declaration set needed to analyze the module internally;
- compute a separate public export table from the facade's explicit exports;
- preserve module identity for diagnostics, caching, semantic analysis, and
  code generation;
- expose only the public export table to importing programs.

The exact Rust data structure is an implementation detail, but the boundary
must be explicit. A single flattened `Program` must not be used as both the
module implementation and the external interface.

### 2. Define the internal module scope

The internal scope contains declarations from the module facade and the
sibling files that the facade opens. Declarations in that scope may refer to
one another according to normal Actus declaration and phase rules.

This scope is used to:

- resolve public wrapper bodies;
- resolve private helpers and unsafe runtime bridges;
- validate types, ownership roles, cleanup, and calls;
- record source spans and declaration provenance;
- prepare the complete implementation unit for code generation.

Internal visibility does not make a declaration public. It only defines what
the module implementation may use while it is compiled as a unit.

### 3. Define the public facade interface

The public interface contains only declarations explicitly exported by the
facade contract:

- a sibling is externally eligible only when the canonical facade opens it;
- a declaration is externally eligible only when it is itself `open` under
  the applicable Actus declaration rules;
- declarations not present in the public export table are not visible to an
  importing source file;
- an `open unsafe extern "C"` bridge is implementation-private by default;
  it does not become a public Actus operation merely because its declaration
  has an ABI boundary;
- typed public wrappers remain public when the facade explicitly exports them.

The interface must carry enough signature information for caller-side type
checking, ownership-role checking, generic checking, and documentation/LSP
features. It must not carry private implementation bodies into the caller's
namespace.

### 4. Separate module analysis from importer analysis

The compiler shall analyze a module unit in two related but distinct views:

1. Analyze the module implementation against its complete internal scope.
2. Analyze the importing source against the module's public interface only.

An imported public declaration may be called, typed, borrowed, moved, or
matched according to its public contract. Its body is not re-resolved against
the caller's scope. Private names referenced by that body are satisfied by
the already-validated module implementation unit.

This preserves the one-directional compiler pipeline:

```text
lexer -> parser -> ast -> semantic -> codegen
```

Module aggregation supplies compilation units and interfaces; it does not
move ownership checking into the parser or make code generation repair an
invalid semantic scope.

### 5. Make private access a deterministic semantic error

An importing program that names a private module declaration must be rejected
before code generation. The diagnostic must identify:

- the requested symbol;
- the imported module;
- the fact that the symbol is not exported by the canonical facade;
- the public facade or exported replacement when one exists.

The implementation shall allocate a stable module-visibility diagnostic in
the existing module diagnostic range (`E1100`–`E1108`) or extend that range in
the normal diagnostic registry if all existing codes are occupied. The code
must be represented in the diagnostic model independently from terminal
rendering and must be deterministic across CLI and LSP consumers.

The compiler must reject direct access to private helpers, raw C-ABI bridges,
private types, and private constants. It must not silently promote them,
duplicate them into the facade, or defer the failure to linking.

### 6. Emit the module implementation, not the private interface

Code generation shall receive the validated module implementation unit. It may
emit the internal functions, declarations, and bridge references required by
public wrappers, while the importer receives only the public interface.

The C ABI remains a runtime boundary, not an Actus visibility escape hatch:

- raw bridge symbols keep their documented ABI and status contracts;
- typed Actus wrappers translate raw statuses into `Result` or other public
  types before crossing the library API boundary;
- code generation must not expose a private bridge as an application-level
  declaration merely because a linker symbol exists;
- duplicate implementation emission must be prevented when several callers
  import the same module.

### 7. Apply the rule consistently to tooling

The same two views apply to language tooling:

- completion and hover outside a module show only public facade exports;
- definition lookup resolves public symbols to their exported declaration and
  may follow the implementation body only as an internal navigation result;
- completion and definition lookup inside a module may include private
  siblings and bridges where the source location has internal access;
- semantic tokens must not classify a private bridge as a public standard
  library API in an importing file;
- unsaved overlays must rebuild the affected module unit and its interface
  deterministically.

## Invariants

1. A facade is the only external gateway to a directory module.
2. Sibling declarations share an internal module scope, not the caller's
   scope.
3. Only explicit facade exports cross the module boundary.
4. Public wrapper bodies resolve private dependencies inside their own module
   unit.
5. Private names cannot be called, imported, constructed, or used in public
   signatures by external source files.
6. Visibility is decided during semantic analysis, never inferred or repaired
   by code generation.
7. Module diagnostics retain source provenance and are independent of their
   renderer.
8. Raw C-ABI status values do not leak through typed standard-library APIs.
9. Repeated imports produce deterministic interfaces and do not duplicate
   generated implementation symbols.
10. The rule applies equally to ordinary modules, standard-library modules,
    overlays, tests, and LSP analysis.

## Compatibility and migration

Existing public Actus APIs remain compatible when their declarations are
already exported through the canonical facade. The migration changes only
incorrectly exposed implementation details:

- direct application calls to private bridges become explicit compiler
  errors;
- standard-library wrappers keep their public signatures and gain a valid
  internal resolution context;
- no C ABI symbol needs to be renamed solely because its Actus declaration
  becomes private;
- source files that intentionally need a raw bridge must use an explicitly
  accepted interop API rather than relying on accidental module flattening.

The Alpha compatibility policy treats accidental private-symbol access as an
invalid program, not as a supported public API. Documentation and migration
notes must identify the typed facade operation that replaces each rejected
raw bridge use.

## Alternatives considered

### Export every parsed declaration

Rejected. It makes unsafe runtime bridges and implementation helpers public,
leaks raw statuses and backend details, and violates the facade contract.

### Remove private declarations before analyzing the module

Rejected. Public wrappers then cannot resolve their legitimate internal
dependencies, producing false semantic errors or encouraging duplicated
facade declarations.

### Hide names by prefix or filename convention

Rejected. Names such as `actus_*` and file placement are not a visibility
model. They are brittle, cannot express future module APIs, and would put
semantic policy into ad-hoc string matching.

### Solve the problem only in the LSP or CLI

Rejected. Visibility is a compiler semantic rule and must be enforced for
every frontend, build mode, test runner, and code-generation entry point.

### Duplicate private bridge declarations in each public wrapper

Rejected. It creates multiple sources of truth, risks ABI drift, and hides the
actual module boundary instead of representing it.

## Implementation gates

1. **Module unit representation:** add internal declaration storage, public
   interface storage, source provenance, and deterministic module identity.
2. **Internal semantic scope:** analyze facade and opened siblings together so
   private helpers and runtime bridges resolve correctly.
3. **Public import interface:** import only exported signatures and preserve
   ownership, generic, and result contracts without importing private bodies.
4. **Visibility diagnostics:** add accepted and rejected cases for private
   calls, private types, and private signature leaks.
5. **Implementation code generation:** emit internal dependencies once and
   validate link symbols without exposing them to callers.
6. **Standard-library migration:** make `std::io`, `std::fs`, and `std::path`
   pass through typed public wrappers while raw bridges remain internal.
7. **LSP and regression matrix:** update completion, hover, definition,
   overlays, CLI checks, application tests, and documentation tests.

## Acceptance criteria

The ADR is implemented when all of the following are true:

- an exported standard-library wrapper compiles while using a private bridge;
- an external source file cannot call that bridge directly;
- a private helper cannot leak through a public signature;
- facade exports remain callable with correct ownership and error contracts;
- code generation emits the required implementation exactly once;
- CLI, native execution, LSP, and negative semantic tests agree on visibility;
- source limits, formatting, Clippy, tests, and documentation checks pass;
- the corresponding Phase 19 visibility checkbox is marked only after these
  tests provide real evidence.

## References

- `AGENTS.md`, Actus module facade and one-directional pipeline rules
- ADR-0018, module and facade architecture
- ADR-0022, typed runtime and C-ABI boundaries
- ADR-0041, strict Actus conformance
- ADR-0045, Alpha core and application readiness
- `src/modules/aggregation/parsing.rs`
- `src/modules/aggregation/exports.rs`
- `src/modules/aggregation/analysis.rs`
- `tests/modules.rs` and `tests/applications.rs`
