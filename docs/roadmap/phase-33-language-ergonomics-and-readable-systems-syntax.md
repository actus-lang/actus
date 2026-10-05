# Phase 33: Language Ergonomics and Readable Systems Syntax

**Status: Planned**

Phase 33 improves Actus source readability without weakening ownership,
borrowing, explicit layout, deterministic cleanup, facade visibility, or native
safety. The goal is to make production Actus code concise and understandable
while keeping every convenience explicit in the compiler's semantic model.

This phase does not introduce project-specific syntax, application behavior, or
implicit runtime allocation. Each convenience must have a defined parser, AST,
semantic, native-lowering, formatter, LSP, diagnostic, and migration contract
before implementation.

## Goals

- Reduce repetitive ownership annotations where the compiler can prove the safe role without ambiguity.
- Provide readable bounded iteration for arrays, ranges, and iterator-like views.
- Replace repeated numeric offsets with named layout declarations.
- Provide small helper forms for repeated, mechanically identical operations.
- Make binary serialization declarations readable and auditable.
- Allow multi-line verb contracts that document behavior near the declaration.
- Allow safe compiler-generated local bindings for role-sensitive expressions.

## Non-goals

- No truthiness, hidden numeric conversion, or silent ownership transfer.
- No unbounded iterators or allocation hidden inside syntactic sugar.
- No compiler-side repair of invalid ownership or borrow states.
- No change to the one-way lexer -> parser -> AST -> semantic -> codegen pipeline.
- No weakening of facade visibility, deterministic cleanup, or native ABI rules.
- No project-specific vocabulary or application-specific lowering.

## Gates

### Gate 33.1 — Safe ownership inference

Make common read-only and call-scoped ownership patterns readable when the
compiler can prove the result uniquely and locally.

#### Design

- [ ] Define the exact contexts where a missing role may be inferred.
- [ ] Limit inference to unambiguous `abs` and `ins` cases initially.
- [ ] Keep `erg` and `dat` explicit whenever mutation, cleanup, or ownership transfer is possible.
- [ ] Define a diagnostic when more than one role is valid or when inference would change the caller binding state.
- [ ] Define an opt-out or explicit-role form for code that requires maximum source-level clarity.
- [ ] Document the inference algorithm and its safety boundary in an ADR.

#### Compiler implementation

- [ ] Add the role-inference representation to the AST without erasing source spans or explicit roles.
- [ ] Resolve inferred roles in semantic analysis before borrow and cleanup validation.
- [ ] Preserve inferred and explicit roles in semantic diagnostics, formatter output, and LSP hover.
- [ ] Reject inference across dynamic dispatch, external ABI calls, and ambiguous overload-like resolution.
- [ ] Ensure native lowering consumes the resolved role and never performs its own inference.
- [ ] Verify deterministic results across repeated compilation.

#### Evidence

- [ ] Add accepted tests for scalar `abs` calls and exclusive `ins` loans.
- [ ] Add rejected tests for buffers, aggregates, resources, `erg`, and `dat`.
- [ ] Add branch-join, early-return, loop, and cleanup regression tests.
- [ ] Add diagnostics tests for ambiguous and unsafe inference.
- [ ] Add formatter and LSP coverage for inferred roles.
- [ ] Record compiler, native, and diagnostic evidence.

### Gate 33.2 — Bounded `for` loops and iterator contracts

Provide readable iteration while preserving static bounds, ownership roles, and
deterministic cleanup.

#### Design

- [ ] Define bounded range syntax with explicit element and index types.
- [ ] Define iteration over fixed arrays, pack-backed arrays, and approved views.
- [ ] Define whether iteration yields a value, an index, an `abs` view, or an `ins` loan for each supported collection.
- [ ] Define behavior for empty ranges, reversed ranges, and overflow.
- [ ] Define whether `break` and `continue` are allowed and how loop-carried ownership joins are checked.
- [ ] Reject unbounded iteration and hidden collection allocation.
- [ ] Document the iterator contract in an ADR.

#### Compiler implementation

- [ ] Add lexer and parser support for the bounded loop form.
- [ ] Add AST nodes that preserve range and collection source spans.
- [ ] Lower `for` loops into the existing verified loop CFG.
- [ ] Reuse existing bounds, cleanup, `break`, and `continue` semantics.
- [ ] Enforce the declared role of each yielded element.
- [ ] Ensure native lowering emits no hidden allocation or floating-point logic.
- [ ] Add formatter and LSP support.

#### Evidence

- [ ] Add accepted tests for arrays, ranges, nested loops, and fixed slots.
- [ ] Add rejected tests for unbounded sources, invalid roles, and overflow.
- [ ] Add cleanup tests for `break`, `continue`, early return, and nested loops.
- [ ] Add native execution tests for indexed and contiguous iteration.
- [ ] Add deterministic diagnostic and formatter tests.
- [ ] Record performance and allocation evidence.

### Gate 33.3 — Named constants and layout declarations

Replace repeated byte offsets, widths, sentinels, and masks with named,
compile-time checked declarations.

#### Design

- [ ] Define a compile-time constant declaration form with explicit type rules.
- [ ] Define named field offsets and layout groups for buffers and packs.
- [ ] Define compile-time arithmetic and overflow behavior.
- [ ] Define whether constants may reference other constants and layout fields.
- [ ] Reject runtime reads, allocation, mutation, and function calls in constants.
- [ ] Preserve facade visibility and nested-facade export rules.
- [ ] Document the layout model and migration rules in an ADR.

#### Compiler implementation

- [ ] Add constant and layout nodes to the AST and formatter.
- [ ] Implement semantic evaluation with checked integer operations.
- [ ] Propagate constant environments through generic specialization and nested facades.
- [ ] Lower constants directly into native values without runtime symbols.
- [ ] Preserve source spans and names in diagnostics and LSP definition lookup.
- [ ] Detect duplicate, cyclic, overflowing, and incompatible declarations.
- [ ] Keep layout declarations consistent with pack size and alignment metadata.

#### Evidence

- [ ] Add tests for scalar constants, masks, offsets, and derived constants.
- [ ] Add tests for pack and buffer layout declarations.
- [ ] Add rejected tests for cycles, overflow, runtime expressions, and private facade access.
- [ ] Add object and executable parity tests with no runtime constant symbols.
- [ ] Add generic and nested-facade regression tests.
- [ ] Record layout and native lowering evidence.

### Gate 33.4 — Readable helper forms for repeated operations

Reduce mechanically duplicated source without hiding control flow, ownership,
or error behavior.

#### Design

- [ ] Identify approved repetition patterns such as fixed-slot selection, repeated field access, and bounded dispatch.
- [ ] Define a helper form that expands to named semantic operations.
- [ ] Require explicit collection bounds and element types.
- [ ] Preserve per-operation source spans for diagnostics.
- [ ] Define when helper expansion is rejected instead of guessed.
- [ ] Prohibit helpers that conceal mutation, ownership transfer, allocation, or external calls.
- [ ] Document expansion and migration behavior.

#### Compiler implementation

- [ ] Add a hygienic AST representation for the helper form.
- [ ] Expand helpers before semantic ownership and bounds analysis.
- [ ] Ensure each generated operation has deterministic ordering.
- [ ] Preserve source-level names in diagnostics, formatter output, and LSP.
- [ ] Verify native lowering is equivalent to the explicit source form.
- [ ] Reject recursive, ambiguous, or side-effectful helper expansion.

#### Evidence

- [ ] Add accepted tests for repeated fixed-slot and field operations.
- [ ] Add rejected tests for invalid bounds, roles, and helper shape.
- [ ] Compare helper and explicit forms at semantic and native output levels.
- [ ] Add diagnostics and source-span regression tests.
- [ ] Add formatter round-trip coverage.
- [ ] Record code-size and runtime parity evidence.

### Gate 33.5 — Declarative serialization contracts

Make binary serialization readable while keeping byte order, offsets, widths,
checksums, versions, and failure behavior explicit.

#### Design

- [ ] Define a serialization declaration for fixed-width fields and sections.
- [ ] Define little-endian, alignment, padding, checksum, and version syntax.
- [ ] Define read, write, validate, and migration operations generated from the contract.
- [ ] Require explicit ownership roles for buffers and paths.
- [ ] Define atomic write and torn-write protection hooks.
- [ ] Reject declarations that produce ambiguous or overlapping fields.
- [ ] Document generated API names and compatibility rules in an ADR.

#### Compiler implementation

- [ ] Add serialization declarations to the parser and AST.
- [ ] Validate field order, offsets, widths, alignment, and total size.
- [ ] Generate typed read/write operations through the existing facade and ABI boundaries.
- [ ] Generate checksum and version validation without hidden allocation.
- [ ] Preserve hand-written escape hatches only through explicit declarations.
- [ ] Emit deterministic native code and stable diagnostics.
- [ ] Add formatter and LSP support for serialization fields.

#### Evidence

- [ ] Add round-trip tests for fixed and dynamic layouts.
- [ ] Add corruption, truncation, checksum, version, and overlapping-field tests.
- [ ] Add atomic commit and staged-file recovery tests.
- [ ] Compare generated serialization with an explicit reference implementation.
- [ ] Add native object and executable parity tests.
- [ ] Record byte-level compatibility evidence.

### Gate 33.6 — Multi-line verb contracts

Provide first-class readable contracts for public and internal verbs.

#### Design

- [ ] Define contract sections for purpose, inputs, outputs, ownership, invariants, errors, side effects, and ABI behavior.
- [ ] Define whether contracts are documentation-only or may contain checked declarations.
- [ ] Keep documentation separate from executable preconditions until a separate design is accepted.
- [ ] Define inheritance and facade visibility for public verb contracts.
- [ ] Define formatter and LSP presentation rules.
- [ ] Reject malformed or contradictory contract sections where parsing is checked.
- [ ] Update the language guide and documentation conventions.

#### Compiler implementation

- [ ] Add structured verb contract nodes while preserving docstring comments.
- [ ] Attach contracts to source spans and exported declarations.
- [ ] Expose contract sections through hover, completion, and definition views.
- [ ] Keep contract parsing independent from semantic ownership validation.
- [ ] Preserve contracts through generic specialization and facade re-exports.
- [ ] Add stable diagnostics for malformed structured contracts.

#### Evidence

- [ ] Add parser tests for complete and partial contract sections.
- [ ] Add formatter round-trip tests.
- [ ] Add LSP hover and documentation extraction tests.
- [ ] Add facade and generic contract visibility tests.
- [ ] Add rejected tests for malformed structured contracts.
- [ ] Record that contracts do not change runtime behavior unless explicitly introduced by a later phase.

### Gate 33.7 — Compiler-generated local bindings

Remove repetitive temporary bindings required only to satisfy role-sensitive
argument expressions, without weakening move and borrow checking.

#### Design

- [ ] Define the exact expression positions where a compiler-generated local may be introduced.
- [ ] Limit the feature to pure scalar expressions with known type and lifetime.
- [ ] Require single evaluation and deterministic left-to-right ordering.
- [ ] Preserve explicit bindings as the canonical escape hatch.
- [ ] Reject buffers, aggregates, resources, calls, mutation, and expressions with observable side effects.
- [ ] Define ownership role assignment for generated locals.
- [ ] Document diagnostics and migration behavior.

#### Compiler implementation

- [ ] Add a semantic normalization pass that materializes approved scalar temporaries before role checking.
- [ ] Preserve original source spans in diagnostics and generated IR metadata.
- [ ] Ensure generated locals cannot escape their call scope.
- [ ] Verify cleanup and drop behavior remains unchanged.
- [ ] Ensure native lowering does not introduce unnecessary allocation or runtime symbols.
- [ ] Add formatter and LSP behavior that does not invent misleading source declarations.
- [ ] Keep the transformation deterministic across object and executable builds.

#### Evidence

- [ ] Add accepted tests for arithmetic, indexing, and offset arguments.
- [ ] Add rejected tests for side effects, aggregates, and ownership escapes.
- [ ] Add use-after-move and borrow-conflict regressions.
- [ ] Compare explicit-binding and generated-binding native output.
- [ ] Add object/executable parity and no-extra-allocation evidence.
- [ ] Record the accepted normalization boundary.

## Cross-cutting acceptance gate

- [ ] Update `ACTUS_CODING_AGENT_GUIDE.md` only with implemented behavior.
- [ ] Add ADRs for ownership inference, bounded iteration, layout declarations, serialization contracts, and generated-local normalization.
- [ ] Update the language guide, style guide, formatter, and LSP documentation.
- [ ] Add accepted and rejected fixtures for every new syntax form.
- [ ] Keep lexer -> parser -> AST -> semantic -> codegen dependency direction.
- [ ] Keep every implementation file below the hard size limit.
- [ ] Keep every function within the preferred limit where practical.
- [ ] Run formatter, compiler check, clippy, tests, source-limit checks, and `git diff --check`.
- [ ] Verify no project-specific application names or implementation details enter the Actus language or standard library.
- [ ] Publish an acceptance report with parser, semantic, native, tooling, performance, and compatibility evidence.

## Completion criteria

Phase 33 is complete only when all seven gates and the cross-cutting acceptance
gate are checked, the required ADRs and documentation are merged, accepted and
rejected behavior is covered, and the full repository quality suite passes.
