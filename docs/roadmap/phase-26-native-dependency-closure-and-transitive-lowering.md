# Phase 26: Native Dependency Closure and Transitive Module Lowering

Phase 26 makes native emission follow the same reachable dependency graph that
semantic analysis already understands. A public verb reached through nested
facades must bring every required private helper, cross-module call, generic
instance, and aggregate return contract into native lowering without widening
the public API.

The target shape is:

```text
root facade
└── nested facade
    └── public verb
        └── private helper
            └── another module helper
```

The compiler must emit the complete reachable native implementation while
keeping the public surface controlled by canonical facades:

```text
public exports -> native dependency closure -> object emission -> linker
```

## Architectural contract

- Native lowering starts from the selected public entry points and computes a
  deterministic transitive call/dependency closure.
- Private helpers may be emitted when reachable, but they must not become
  facade exports, external package symbols, or public API declarations.
- Nested facade traversal must preserve the existing resolver, visibility, and
  module-object boundaries.
- Generic verbs reached through the closure must be specialized for every
  concrete instance required by the call graph.
- Aggregate return types and their layouts must be registered before function
  declaration and lowering; return-slot ABI contracts must remain consistent.
- The closure must be cycle-safe, deterministic, and deduplicated by canonical
  module, declaration, and specialization identity.
- Semantic validation remains authoritative. Native lowering must not invent
  missing declarations or bypass visibility checks.
- No private helper is made `open` merely to make native emission succeed.

## Gate 26.0: Dependency-closure contract

- [x] Define the native dependency graph nodes and edges for verbs, generic
      instances, aggregate types, performances, and external bridges.
- [x] Define the root set for hosted entry verbs, configured freestanding
      entries, object emission, and test-runner entry points.
- [x] Define deterministic identity and ordering for reachable dependencies.
- [x] Define cycle handling and diagnostics for recursive or invalid native
      dependency paths.
- [x] Record that native reachability never changes facade visibility.
- [x] Add architecture fixtures for a public verb, private helper, nested
      facade, and cross-module helper chain.

Gate 26.0 is closed as the dependency-closure contract gate. The contract
defines public/native roots, reachable declaration categories, canonical
identity and ordering, cycle/duplicate handling, and the rule that native
reachability never widens facade visibility. The fixtures in
`tests/fixtures/native_dependency_closure/phase-26-contract.txt` cover a
public-to-private transitive chain, repeated-helper deduplication, private
facade bypass, unresolved reachable helpers, and duplicate native identity.
Gates 26.1-26.6 remain open until the compiler implements and executes this
contract with native evidence.

## Gate 26.1: Reachable call-graph discovery

- [x] Traverse direct calls from every selected public/native root.
- [x] Recursively discover private helpers in the same module object.
- [x] Resolve calls across nested facade-owned module objects through the
      canonical module identity.
- [x] Include method/performance dispatch dependencies required by the native
      implementation.
- [x] Include external bridge declarations without treating them as Actus
      function bodies.
- [ ] Reject unresolved reachable calls before object emission with a stable
      diagnostic.
- [x] Deduplicate repeated helper calls and preserve deterministic traversal.

Gate 26.1 implementation is in place for reachable Actus verbs, private
helpers, method calls, performances, external bridge declarations, and stable
deduplicated traversal. The unresolved-reachable-call diagnostic remains open
for Gate 26.5 because it requires source-span propagation and parity across
check, build, test, and LSP diagnostics.

## Gate 26.2: Generic and transitive specialization

- [x] Propagate concrete generic arguments through nested helper calls.
- [x] Materialize all reachable `N: Usize` const-generic instances before
      native declaration and lowering.
- [x] Propagate type substitutions through call arguments, return types,
      indexed places, and nested aggregate fields.
- [x] Reuse the existing generic instance identity/cache contract.
- [x] Prevent unreachable generic declarations from being emitted.
- [x] Add accepted tests for nested generic calls across module
      and facade boundaries.
- [ ] Add rejected tests for unresolved or ambiguous nested generic calls with
      stable diagnostics.

Gate 26.2 now materializes every concrete generic verb instance discovered by
the semantic instance graph, rewrites const-generic expressions and nested
calls to deterministic specialized names, and registers those names across
module-object symbol bindings. Native regressions cover multiple instances in
one call graph and imported and nested facade boundaries. Rejected-call
diagnostics remain open for Gate 26.5 because they require source-span
propagation and parity across check, build, test, and LSP diagnostics.

## Gate 26.3: Aggregate return and layout dependencies

- [x] Register reachable struct, pack, enum, array, and option/result layouts
      before lowering dependent functions.
- [x] Register caller return slots and callee return lowering consistently for
      aggregate return types.
- [x] Preserve array-backed pack stride, alignment, ownership, and copy/move
      behavior across helper calls.
- [x] Ensure aggregate dependencies are included even when they are referenced
      only by a return type or specialized signature.
- [x] Add native tests for scalar, array, struct, pack, and generic aggregate
      returns through transitive helpers.

Gate 26.3 is covered by native return-slot and layout evidence. The
transitive aggregate regression exercises scalar and generic helper returns,
plain arrays, structs, and array-backed packs; it also verifies that a
specialized `Array[T, N]()` constructor receives the concrete capacity before
native lowering. Existing enum and option/result layout tests remain part of
the aggregate ABI coverage.

## Gate 26.4: Object emission and symbol boundaries

- [ ] Emit each reachable private helper exactly once in its owning native
      object or the defined shared object boundary.
- [ ] Keep private helper symbols internal or deterministically namespaced.
- [ ] Preserve public facade symbols and external bindings without exposing
      private implementation declarations.
- [ ] Link root and imported objects using the computed closure metadata.
- [ ] Reject duplicate native identities before linking.
- [ ] Verify that empty or unreachable modules do not produce spurious runtime
      symbols.

## Gate 26.5: Tooling and diagnostics parity

- [ ] Make strict check, test runner, object build, executable build, and LSP
      use the same dependency-resolution contract where applicable.
- [ ] Report unresolved native dependencies with the originating call span and
      actionable module/helper identity.
- [ ] Keep formatter and source-limit checks independent of native reachability.
- [ ] Add regression coverage for malformed nested facades, missing helpers,
      generic mismatch, duplicate symbols, and private-export bypass attempts.
- [ ] Document the dependency-closure contract in the coding-agent guide and
      compiler architecture documentation after implementation.

## Gate 26.6: End-to-end acceptance

- [ ] `actus check --strict` accepts the nested facade dependency fixture.
- [ ] `actus test --strict` executes tests using private transitive helpers.
- [ ] `actus build --strict --emit obj` emits all reachable objects and symbols.
- [ ] `actus build --strict --emit exe` links and executes the fixture.
- [ ] Nested generic helper chains execute with the expected concrete values.
- [ ] Aggregate return helpers execute without invalid return-slot or layout
      behavior.
- [ ] Private helpers remain absent from the public facade/API surface.
- [ ] Repeated builds, object identities, diagnostics, and dependency ordering
      are deterministic.
- [ ] Full Rust formatting, check, clippy, test, source-limit, documentation,
      architecture, and diff checks pass.

## Non-goals

This phase does not:

- make private helpers public or add implicit `open` declarations;
- change Actus ownership, borrowing, generic syntax, or facade visibility;
- introduce a new runtime linker or dynamic loading model;
- replace semantic analysis with codegen-time inference;
- add AIE-specific syntax or special-case module names;
- change the package configuration contract defined by Phase 25.

## Completion criteria

Phase 26 is complete only when native emission is a deterministic closure of
the selected public roots, every reachable private and generic dependency is
lowered exactly once, aggregate ABI/layout dependencies are registered before
use, and no implementation detail leaks into the public facade surface. The
result must be demonstrated by strict semantic, native object, executable,
runtime, visibility, and negative regression evidence.
