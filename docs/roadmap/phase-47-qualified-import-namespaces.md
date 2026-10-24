# Phase 47 — Qualified Import Namespaces

## Status

Complete. This phase introduces explicit module-qualified name lookup while
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
- [x] Define how aliases, nested facades, package roots, and imports that
      resolve to non-facade children behave.
- [x] Define stable diagnostics for malformed, unresolved, ambiguous, and
      private qualified paths.
- [x] Add parser and semantic negative tests before backend work begins.

### Gate 47.2 — Shared semantic lookup

- [x] Add an imported module namespace binding without ambient name injection.
- [x] Resolve qualified declarations through canonical facades only.
- [x] Preserve ownership roles and generic/type arguments after qualification.
- [x] Reject direct implementation-child access and ambiguous aliases.
- [x] Add visibility, collision, alias, nested-facade, and package-boundary tests.

Evidence: module resolution and semantic tests pass 83/83. Qualified verbs,
aliases, constants, enum types/variants, generic calls, explicit `open`
imports, private-child rejection, and import-collision diagnostics are covered.
Cranelift/LLVM lowering parity passes 8/8 for the qualified constant and enum
fixtures.

### Gate 47.3 — Backend-neutral lowering parity

- [x] Carry the resolved declaration identity through shared lowering.
- [x] Verify qualified constants, verbs, generic calls, enum variants, and
      aggregate constructors through the shared lowering boundary.
- [x] Add Cranelift/LLVM parity tests for accepted and rejected programs.
- [x] Confirm no backend-specific qualified-name lookup or fallback exists.
- [x] Retain zero-float native verification for affected executable fixtures.

Evidence: the shared lowering suite passes qualified constant, verb, generic,
enum, and aggregate fixtures on both Cranelift and LLVM (10/10 parity tests).
The strict LLVM build passes and the existing freestanding zero-float evidence
remains green.

### Gate 47.4 — Formatter, LSP, and diagnostics

- [x] Format qualified paths deterministically without converting them to
      `open` declarations.
- [x] Add LSP completion, go-to-definition, hover, and diagnostic spans for
      imported namespaces.
- [x] Preserve qualified paths through parse-format-parse round trips.
- [x] Add tests for unresolved modules, missing declarations, aliases, and
      private-child diagnostics in editor output.

Evidence: formatter regression coverage keeps qualified import paths stable and
keeps `open` imports inside a legal import block. LSP regression coverage
resolves completion members, aliases, hover, and go-to-definition through an
imported namespace, and reports unresolved, missing-member, and private-child
diagnostics with qualified names and spans.

### Gate 47.5 — Standard-library migration inventory

- [x] Inventory every standard-library `open` declaration and classify it as
      migration candidate, intentional local opening, or blocked by a missing
      compiler capability.
- [x] Migrate selected standard-library consumers to qualified access only
      after the shared semantic and backend gates pass.
- [x] Keep migration changes separate from compiler implementation changes.
- [x] Document remaining `open` uses and their reasons.

Inventory evidence (2026-10-24): the standard library contains 346 `open`
declarations. 280 are public declaration exports (`open verb`, `open struct`,
`open enum`, `open const`, and related API declarations). The remaining 65 are
module-level re-exports through canonical facades, including the root standard
library facade and nested `io`, `path`, `fs`, `wire`, `region`, `string`,
`time`, `process`, and `charmap` facades. `fs/directory.act` and
`fs/operations.act` also retain intentional re-exports because their public
signatures expose the shared `Path`, `File`, and `IoError` API types. No
module-level `open` was classified as a safe removal candidate; removing any
of the inventoried declarations would either hide public API or bypass the
canonical facade contract.

Migration evidence (2026-10-24): `library/std/src/fs/options.act` now imports
the `std::io` and `std::path` namespaces without opening them and uses
`io::IoError` and `path_module::Path` in its public contract. The compiler
aggregation regression test covers this facade-signature case. Remaining
module-level `open` declarations are retained for canonical facade re-exports;
the other filesystem consumers remain follow-up candidates until their
qualified signatures are migrated and tested independently.

Change-boundary evidence (2026-10-24): compiler/parser/semantic changes,
backend lowering, standard-library migration, and roadmap documentation were
committed as separate thematic commits. No standard-library migration is
mixed into the compiler implementation commit.

Compatibility evidence (2026-10-24):
`legacy_open_and_qualified_import_share_one_imported_module_identity` verifies
that a legacy `open` consumer and a qualified `import` consumer both analyze
successfully and resolve the same imported module identity.

### Gate 47.6 — Compatibility and release evidence

- [x] Prove existing `open` programs remain valid during the migration window.
- [x] Prove qualified and legacy forms resolve to the same declaration identity
      where both are accepted.
- [x] Run formatter checks, lock validation, strict compiler checks, full tests,
      and native backend parity suites.
- [x] Record compiler revision, target profiles, and known limitations.
- [x] Update ADR-0087 from Proposed only after all accepted evidence exists.

Release evidence (2026-10-24): compiler revision `7daf9fd7` contains the
qualified-import implementation and release evidence. `cargo fmt --check`,
`git diff --check`, root and project lock validation, strict source checks, and
`cargo test --quiet` pass. `cargo test --all-features --quiet` passes with both
Cranelift and LLVM enabled; the qualified backend-parity suite passes 10/10,
and strict native fixtures report no floating-point instructions. The tested
profiles are hosted x86_64 Linux debug builds with Cranelift (default) and
LLVM (`--all-features`). Known limitations are intentional: directory-facade
`open` re-exports remain public, external consumers must use the unified
`import { open ... }` form, and no automatic migration or removal of facade
exports is implied.

## Definition of done

Phase 47 is complete only when qualified imports are parsed, semantically
resolved, lowered identically by Cranelift and LLVM, supported by formatter and
LSP tooling, and covered by compatibility and visibility evidence. No `open`
removal is implied until a separate migration decision is accepted.

## Follow-up: canonical consumers inside nested module trees

The compiler capability is complete, but Actus module trees still need an
explicit consumer migration pass. Sibling implementation modules under one
canonical parent must be able to refer to one another through complete module
paths, without relying on ambiguous ambient openings. This follow-up is part
of the Actus module and facade contract, separate from compiler implementation.

- [ ] Replace consumer-side ambiguous `open` entries with canonical imports
      such as `import parent::child;` where qualified ownership is required.
- [ ] Use `child::...` at call sites so the owning nested module remains
      visible.
- [ ] Keep `open sibling;` only where it is an intentional canonical-facade
      re-export; remove it from ordinary nested consumers.
- [ ] Add explicit aliases when nested imports share a final segment, such as
      `import parent::child as local_child;` and
      `import other::child as other_child;`.
- [ ] Add a regression fixture proving nested sibling imports, qualified
      types, and same-final-segment aliases across one canonical parent.
- [ ] Record compiler revision, formatter output, strict checks, and the
      resulting facade/module-visibility evidence before marking this
      follow-up complete.
