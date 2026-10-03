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

- [x] Discover child directories as module members without treating them as
      unrelated package roots.
- [x] Require `layout/layout.act`, `runtime/runtime.act`, and equivalent
      canonical facades for every declared child module.
- [x] Produce deterministic module paths and source ordering independent of
      host filesystem enumeration order.
- [x] Reject ambiguous file/directory module shapes and missing child facades
      with stable diagnostics.
- [x] Preserve existing package roots, dependency roots, runtime modules, and
      facade-bypass protections.

Gate 24.1 is closed. `ResolvedModule` now exposes deterministic child-module
metadata with canonical module paths, directories, and facades. Child
directories require a matching `<child>/<child>.act` facade; a missing facade
uses the existing `E1101` contract, while a direct sibling-file conflict uses
the existing `E1102` ambiguity contract. Direct child imports continue to be
rejected through the parent facade boundary with `E1108`.

Evidence is provided by the resolver suite, including sorted child discovery,
missing-child-facade rejection, sibling/child ambiguity rejection, and the
existing package-root and facade-bypass regressions.

## Gate 24.2: Parent-to-child facade aggregation

- [x] Add explicit parent-facade aggregation for child modules.
- [x] Ensure declarations exposed by a child facade are visible through the
      parent only when the parent explicitly opens that child.
- [x] Preserve child internal scope across all implementation siblings.
- [x] Reject unknown child names, duplicate child exports, and collisions
      between parent and child declarations before code generation.
- [x] Keep private child declarations inaccessible to external consumers.

Gate 24.2 is closed: parent parsing and export aggregation now follows explicit
child-facade openings recursively. A child facade's public declarations are
available through the parent facade, while declarations kept private in child
implementation siblings remain available only to the aggregated internal
scope. External resolution still uses the parent boundary, so opening a child
internally does not make the child facade directly importable. The module test
suite provides 51 passing tests for this behavior, including deterministic
rejection of duplicate child exports and parent-child export collisions before
code generation.

## Gate 24.3: Namespace and visibility boundaries

- [x] Permit external code to import the parent module without importing child
      paths directly.
- [x] Reject direct child imports that bypass a parent facade.
- [x] Preserve deterministic namespace identity for parent and child modules.
- [x] Validate generic types, generic verbs, packs, constants, performances,
      and nested calls across the parent-child boundary.
- [x] Add accepted and rejected ownership, visibility, and duplicate-symbol
      tests for hierarchical modules.

Gate 24.3 is closed. External callers resolve the parent facade as the only
public gateway; direct child paths continue to produce the existing bypass
diagnostic. Namespace identities and symbol prefixes remain deterministic and
distinct for parent and child paths. Acceptance coverage includes generic
types, generic verbs with nested calls, packs, constants, roles, and
performances. Rejection coverage includes private child packs and constants,
private types and roles in exported signatures, duplicate exports, parent-child
collisions, and direct child imports. The module suite provides 56 passing
tests for these boundaries.

## Gate 24.4: Compiler pipeline integration

- [x] Make parsing, semantic analysis, generic discovery, native object
      planning, and linking consume the same hierarchical module graph.
- [x] Ensure imported child implementations are emitted exactly once and
      parent facades do not create duplicate object owners.
- [x] Preserve transitive generic specialization across child modules,
      including const-generic values and nested generic calls.
- [x] Keep backend-specific types out of resolver, parser, AST, and semantic
      module boundaries.
- [x] Add native execution tests for a parent facade that re-exports code from
      multiple child directories.

Gate 24.4 is closed: hierarchical child sources are now part of the owning
`ModuleUnit` source identity and are loaded in deterministic facade/sibling
order. Object planning keeps the complete child implementation under the
parent module owner, so a child directory does not create a second object
owner or duplicate emission. The module suite provides 58 passing tests for
this boundary. Native acceptance proves a const-generic nested call chain
through a child facade (`outer[N] -> inner[N]`) with executable exit code `4`,
and a package with multiple child facades links and runs with exit code `42`.

## Gate 24.5: Tooling parity

- [x] Teach the formatter to preserve and format parent-child facade
      declarations deterministically.
- [x] Make LSP diagnostics, definitions, references, hover, semantic tokens,
      and document symbols resolve declarations through the parent facade.
- [x] Ensure malformed or partially edited child modules do not terminate the
      language server.
- [x] Make the test runner discover complete module trees rather than treating
      child implementation files as independent test roots.
- [x] Add formatter idempotence and LSP integration fixtures for nested
      facades.

Gate 24.5 is closed. LSP definition, hover, references, and rename lookups
consume the complete hierarchical `ModuleUnit` source list, so declarations
opened through nested child facades resolve to their actual implementation
files. The tooling fixtures cover parent imports, child-facade navigation,
formatter idempotence, and malformed child overlays; the language server keeps
serving diagnostics and requests while an edited child is temporarily invalid.
The test runner maps nested tests to their highest canonical parent facade and
executes them with the complete child sibling scope.

## Gate 24.6: Documentation and developer workflow

- [x] Document hierarchical module layout, canonical child facades, and
      parent-controlled exports in the language guide.
- [x] Document valid and invalid import paths with examples.
- [x] Add a production-shaped example using a single public facade and
      responsibility-specific child directories.
- [x] Update diagnostics and module architecture documentation without
      contradicting the existing direct-sibling rules.
- [x] Add an executable module-tree example to the repository acceptance
      suite.

Gate 24.6 is closed. The Alpha guide and coding-agent guide document
canonical nested facades, explicit parent-controlled exports, and valid and
invalid import paths. The compiler architecture documentation records the
shared module-unit identity used across resolution, semantic analysis, native
planning, formatter, LSP, and test tooling. The
`examples/hierarchical_modules` package provides a production-shaped parent
facade with responsibility-specific child directories, and its acceptance
test proves strict checking, formatting, native linking, and execution with
exit code `24`.

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
