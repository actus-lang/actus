# Phase 24: Hierarchical Modules and Nested Facades

Phase 24 extends Actus modules from one directory with direct sibling sources
to a deterministic hierarchy of parent modules and child directory modules.
The goal is to let a package expose one stable public facade while keeping
large implementations split by domain responsibility.

The target shape is:

```text
src/aie/
├── aie.act                 # public parent facade
├── layout/
│   └── layout.act          # child facade
├── runtime/
│   ├── runtime.act         # child facade
│   ├── fabric.act
│   ├── coincidence.act
│   ├── forwarding.act
│   ├── neurogenesis.act
│   └── decay.act
└── persistence/
    └── persistence.act     # child facade
```

External consumers import only the parent module:

```act
import aie;
```

The parent facade explicitly controls which child declarations become part of
the public `aie` namespace. A child module must not become independently
reachable merely because its directory exists.

## Architectural contract

- Every directory module has a canonical facade whose filename matches the
  directory name.
- A parent facade is the only public gateway to its child modules.
- Child modules may contain direct sibling implementation files and may expose
  only declarations marked `open`.
- Parent aggregation is explicit and deterministic; filesystem enumeration
  never determines the public API.
- Direct imports that bypass an existing parent facade remain rejected.
- Module resolution, visibility, diagnostics, formatter behavior, LSP
  behavior, and test-runner behavior must agree on the same hierarchy.
- This phase changes module organization only. It does not add target-specific
  syntax, implicit imports, or a second visibility system.

## Gate 24.0: Baseline and module-tree contract

- [x] Add positive and negative fixtures for a parent facade with child
      directory modules.
- [x] Define the accepted filesystem shape, canonical facade rule, and
      diagnostics for missing or ambiguous child facades.
- [x] Define whether an empty child directory, a child facade without
      implementation siblings, and a nested child hierarchy are accepted.
- [x] Record the resolver, aggregation, visibility, parser, LSP, formatter,
      and test-runner ownership before implementation.
- [x] Keep the implementation target-neutral and below repository file-size
      and function-size limits.

Gate 24.0 is closed as a baseline gate. The resolver tests record that a
parent facade currently resolves only its direct `.act` siblings and does not
yet discover child directories. They also preserve the existing `E1108`
diagnostic for a direct child-module import that bypasses the parent facade.
The implementation work begins at Gate 24.1.

## Gate 24.1: Hierarchical resolver

- [ ] Discover child directories as module members without treating them as
      unrelated package roots.
- [ ] Require `layout/layout.act`, `runtime/runtime.act`, and equivalent
      canonical facades for every declared child module.
- [ ] Produce deterministic module paths and source ordering independent of
      host filesystem enumeration order.
- [ ] Reject ambiguous file/directory module shapes and missing child facades
      with stable diagnostics.
- [ ] Preserve existing package roots, dependency roots, runtime modules, and
      facade-bypass protections.

## Gate 24.2: Parent-to-child facade aggregation

- [ ] Add explicit parent-facade aggregation for child modules.
- [ ] Ensure declarations exposed by a child facade are visible through the
      parent only when the parent explicitly opens that child.
- [ ] Preserve child internal scope across all implementation siblings.
- [ ] Reject unknown child names, duplicate child exports, and collisions
      between parent and child declarations before code generation.
- [ ] Keep private child declarations inaccessible to external consumers.

## Gate 24.3: Namespace and visibility boundaries

- [ ] Permit external code to import the parent module without importing child
      paths directly.
- [ ] Reject direct child imports that bypass a parent facade.
- [ ] Preserve deterministic namespace identity for parent and child modules.
- [ ] Validate generic types, generic verbs, packs, constants, performances,
      and nested calls across the parent-child boundary.
- [ ] Add accepted and rejected ownership, visibility, and duplicate-symbol
      tests for hierarchical modules.

## Gate 24.4: Compiler pipeline integration

- [ ] Make parsing, semantic analysis, generic discovery, native object
      planning, and linking consume the same hierarchical module graph.
- [ ] Ensure imported child implementations are emitted exactly once and
      parent facades do not create duplicate object owners.
- [ ] Preserve transitive generic specialization across child modules,
      including const-generic values and nested generic calls.
- [ ] Keep backend-specific types out of resolver, parser, AST, and semantic
      module boundaries.
- [ ] Add native execution tests for a parent facade that re-exports code from
      multiple child directories.

## Gate 24.5: Tooling parity

- [ ] Teach the formatter to preserve and format parent-child facade
      declarations deterministically.
- [ ] Make LSP diagnostics, definitions, references, hover, semantic tokens,
      and document symbols resolve declarations through the parent facade.
- [ ] Ensure malformed or partially edited child modules do not terminate the
      language server.
- [ ] Make the test runner discover complete module trees rather than treating
      child implementation files as independent test roots.
- [ ] Add formatter idempotence and LSP integration fixtures for nested
      facades.

## Gate 24.6: Documentation and developer workflow

- [ ] Document hierarchical module layout, canonical child facades, and
      parent-controlled exports in the language guide.
- [ ] Document valid and invalid import paths with examples.
- [ ] Add a production-shaped example using a single public facade and
      responsibility-specific child directories.
- [ ] Update diagnostics and module architecture documentation without
      contradicting the existing direct-sibling rules.
- [ ] Add an executable module-tree example to the repository acceptance
      suite.

## Gate 24.7: End-to-end acceptance

- [ ] `actus check --strict` accepts the hierarchical module fixture.
- [ ] `actus test --strict` runs parent and child module tests with complete
      sibling visibility.
- [ ] Native object and executable builds emit deterministic artifacts with no
      duplicate module objects.
- [ ] Parent-facade public API and child-private API are both tested.
- [ ] Full Rust formatting, check, clippy, test, source-limit, documentation,
      architecture, and diff checks pass.
- [ ] Repeated builds and repeated diagnostics produce identical results.

## Non-goals

This phase does not:

- add implicit imports or wildcard filesystem exports;
- allow direct child-module imports that bypass a parent facade;
- change ownership, borrowing, cleanup, ABI, or generic semantics;
- add CPU, MCU, operating-system, AIE, Wire, or Ustari-specific language
  constructs;
- make every implementation file independently importable;
- replace explicit facade visibility with directory-name conventions alone.

## Completion criteria

Phase 24 is complete only when a package can expose one parent facade while
organizing implementation into nested, responsibility-specific child modules,
and the same module graph is honored by resolver, semantic analysis, native
code generation, formatter, LSP, test runner, diagnostics, and documentation.
The result must be proven with accepted and rejected fixtures and complete
repository quality checks; a directory layout that merely exists on disk is
not evidence of completion.
