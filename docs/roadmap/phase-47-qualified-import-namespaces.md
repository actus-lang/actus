# Phase 47 — Qualified Import Namespaces

## Status

Planned. This phase introduces explicit module-qualified name lookup while
preserving the existing `open` form during migration. It applies only to the
Actus compiler, standard library, formatter, and language tooling; it does not
contain application-specific Twin-e behavior.

## Objective

Make module ownership visible at use sites and reduce ambient namespace
collisions without weakening canonical facade or private-child visibility.

The target model is:

```text
import facade
    -> named module binding
    -> qualified lookup
    -> canonical public declaration
    -> shared semantic identity
        -> Cranelift
        -> LLVM
```

## Architectural source of truth

- [ADR-0087](../decisions/ADR-0087-qualified-import-namespaces.md) defines
  qualified lookup, migration, and rejected alternatives.
- [ADR-0047](../decisions/ADR-0047-module-internal-scope-and-facade-visibility.md)
  defines canonical facades and private-child visibility.
- [ADR-0018](../decisions/ADR-0018-directory-modules-facade-contracts-and-intra-module-scoping.md)
  defines directory-module and facade boundaries.
- [ADR-0054](../decisions/ADR-0054-package-configuration-and-compile-time-modules.md)
  defines package and compile-time module resolution.

## Required invariants

- `import` does not implicitly open declarations into unqualified scope.
- Qualified access resolves only through the imported canonical facade.
- Private implementation children remain inaccessible from consumers.
- Existing `open` behavior remains explicit and source-compatible during the
  migration window.
- All semantic identity, ownership, type, and visibility decisions are made
  before either native backend lowers a call or value.
- Cranelift and LLVM receive identical qualified-name resolution results.
- Formatter and LSP output preserve the same qualified path structure.

## Gates

### Gate 47.1 — Grammar and module-resolution contract

- [x] Record the qualified-path grammar and reserved-token decision in the
      parser contract.
- [x] Define the AST node and source-span behavior for one or more path
      components.
- [ ] Define how aliases, nested facades, package roots, and imports that
      resolve to non-facade children behave.
- [ ] Define stable diagnostics for malformed, unresolved, ambiguous, and
      private qualified paths.
- [ ] Add parser and semantic negative tests before backend work begins.

### Gate 47.2 — Shared semantic lookup

- [ ] Add an imported module namespace binding without ambient name injection.
- [ ] Resolve qualified declarations through canonical facades only.
- [ ] Preserve ownership roles and generic/type arguments after qualification.
- [ ] Reject direct implementation-child access and ambiguous aliases.
- [ ] Add visibility, collision, alias, nested-facade, and package-boundary tests.

### Gate 47.3 — Backend-neutral lowering parity

- [ ] Carry the resolved declaration identity through shared lowering.
- [ ] Verify qualified constants, verbs, generic calls, enum variants, and
      aggregate constructors through the shared lowering boundary.
- [ ] Add Cranelift/LLVM parity tests for accepted and rejected programs.
- [ ] Confirm no backend-specific qualified-name lookup or fallback exists.
- [ ] Retain zero-float native verification for affected executable fixtures.

### Gate 47.4 — Formatter, LSP, and diagnostics

- [ ] Format qualified paths deterministically without converting them to
      `open` declarations.
- [ ] Add LSP completion, go-to-definition, hover, and diagnostic spans for
      imported namespaces.
- [ ] Preserve qualified paths through parse-format-parse round trips.
- [ ] Add tests for unresolved modules, missing declarations, aliases, and
      private-child diagnostics in editor output.

### Gate 47.5 — Standard-library migration inventory

- [ ] Inventory every standard-library `open` declaration and classify it as
      migration candidate, intentional local opening, or blocked by a missing
      compiler capability.
- [ ] Migrate selected standard-library consumers to qualified access only
      after the shared semantic and backend gates pass.
- [ ] Keep migration changes separate from compiler implementation changes.
- [ ] Document remaining `open` uses and their reasons.

### Gate 47.6 — Compatibility and release evidence

- [ ] Prove existing `open` programs remain valid during the migration window.
- [ ] Prove qualified and legacy forms resolve to the same declaration identity
      where both are accepted.
- [ ] Run formatter checks, lock validation, strict compiler checks, full tests,
      and native backend parity suites.
- [ ] Record compiler revision, target profiles, and known limitations.
- [ ] Update ADR-0087 from Proposed only after all accepted evidence exists.

## Definition of done

Phase 47 is complete only when qualified imports are parsed, semantically
resolved, lowered identically by Cranelift and LLVM, supported by formatter and
LSP tooling, and covered by compatibility and visibility evidence. No `open`
removal is implied until a separate migration decision is accepted.
