# Phase 31: Native Constant Resolution Through Canonical Facades

## Status

Planned. Phase 31 implements [ADR-0058](../decisions/ADR-0058-native-constant-resolution-through-facades.md).
The phase addresses the compiler boundary where semantic analysis resolves a
facade-exported compile-time constant but native lowering cannot yet materialize
the same value in a nested module or generic specialization.

This phase is compiler-only. It must not add application-specific fixtures,
neural-network code, tokenization code, or external project sources to the
Actus repository.

## Objective

Make compile-time constants reachable during native lowering through the same
canonical facade dependency graph used by semantic analysis, without creating
runtime symbols, weakening visibility, changing ownership roles, or using
source-level literal workarounds.

## Gate 31.0: Contract and module-graph inventory

- [x] Adopt ADR-0058 as the governing decision for this phase.
- [x] Identify the existing constant declaration, export, aggregation,
      normalization, generic specialization, and native-emission boundaries.
- [x] Document the single shared representation of a resolved constant's
      declaration identity, type, initializer, provenance, and source span.
- [x] Confirm that canonical parent facades remain the only external gateway
      to nested modules.
- [x] Add a neutral minimal reproducer covering strict checking and native
      emission for an exported nested constant.

### Gate 31.0 evidence

- Reproducer `canonical_parent_facade_exposes_nested_configuration_constant`
  covers `config -> feature facade -> implementation -> consumer`, strict
  checking, native emission, and the expected executable result.
- The ADR records the observed native-binding failure shape; existing
  configuration tests cover accepted scalar, aggregate, predicate, and nested
  module cases.
- No changes outside the Actus compiler tests and required documentation.

## Gate 31.1: Facade-exported constant environment

- [x] Build a deterministic constant environment from legal module exports.
- [x] Traverse child facades only through their canonical parent exports.
- [x] Preserve declaration provenance so same-name constants from different
      modules cannot be silently conflated.
- [x] Include only public constants visible to the consuming compilation unit.
- [x] Reject duplicate or incompatible exported constants deterministically.
- [x] Keep private constants out of external environments.

### Gate 31.1 evidence

- Accepted nested-facade constant fixture.
- Rejected private-constant and duplicate-export fixtures.
- Rejected direct-child-facade-bypass fixture.
- `imported_object_program_carries_only_facade_exported_constants` proves that
  the imported native object program carries `BUFFER_STRIDE` through the
  `config -> feature` dependency edge while excluding `PRIVATE_STRIDE`.
- Existing module aggregation tests cover duplicate exports, private
  constants, and direct child-facade bypass diagnostics.

## Gate 31.2: Native lowering parity

- [x] Pass the resolved constant environment into native normalization and
      identifier lowering.
- [x] Substitute a typed compile-time initializer before an identifier can be
      treated as a runtime/native binding.
- [x] Preserve the declared constant type and source span during substitution.
- [x] Ensure object and executable emission use the same environment and
      produce the same semantic result.
- [x] Ensure constants do not allocate storage or create external ABI symbols.
- [x] Replace the internal missing-native-binding failure with the correct
      semantic/visibility diagnostic when the constant is not reachable.

### Gate 31.2 evidence

- Scalar return and scalar initializer native tests.
- Predicate/branch-condition native test.
- Aggregate-field and pack-field native tests.
- Object and executable output both pass.
- `nested_facade_constant_has_object_and_executable_parity` verifies the
  scalar initializer, predicate, aggregate field, executable result `24`, and
  absence of a `BUFFER_STRIDE` runtime symbol in the imported object.

## Gate 31.3: Generic specialization propagation

- [x] Carry the resolved constant environment into generic verb instances.
- [x] Resolve constants inside generic bodies after specialization without
      falling back to the unspecialized identifier.
- [x] Preserve const-generic arguments and ordinary type arguments separately.
- [x] Support a generic nested module whose body returns or consumes an
      exported facade constant.
- [x] Cache equivalent specialized constant environments deterministically.
- [x] Reject incomplete or conflicting environments before native emission.

### Gate 31.3 evidence

- Generic nested-facade acceptance test with a concrete executable result.
- Generic scalar, predicate, and aggregate-use coverage.
- Repeated specialization produces no duplicate native declaration.
- `generic_facade_constant_survives_scalar_predicate_and_aggregate_specialization`
  verifies constant substitution together with `N` specialization and returns
  the expected executable result `26`.

## Gate 31.4: Native dependency and symbol integrity

- [x] Verify that compile-time constants are not added to runtime dependency
      roots or native symbol registries.
- [x] Verify that one constant reference remains one compile-time value even
      when several facades expose the same declaration.
- [x] Reject incompatible external/native declarations without weakening the
      existing symbol-collision rules.
- [x] Keep object symbol tables free of duplicate constant symbols.
- [x] Compare repeated object and executable emission for deterministic output.

### Gate 31.4 evidence

- Object-symbol inspection with no generated constant ABI symbol.
- Duplicate-reference and repeated-build tests.
- Existing native ABI and external-symbol regression suites remain green.
- `facade_constant_objects_are_deterministic_and_symbol_free` compares
  repeated object output, rejects duplicate symbols, and verifies that
  `BUFFER_STRIDE` has no runtime/native symbol.

## Gate 31.5: Tooling and diagnostic parity

- [ ] Make strict check, test runner, object build, executable build, and LSP
      use the same visibility and constant-resolution contract where relevant.
- [ ] Preserve hover, definition, and diagnostics for exported and private
      constants.
- [ ] Add formatter/LSP fixtures for canonical facade constant imports.
- [ ] Keep diagnostics deterministic for inaccessible, duplicate, and
      type-incompatible constants.
- [ ] Update the language guide and compiler architecture documentation only
      for behavior that is implemented and tested.

### Gate 31.5 evidence

- Strict-check, test-runner, native, and LSP parity tests.
- Accepted and rejected diagnostics compared across repeated runs.

## Gate 31.6: Quality, architecture, and regression acceptance

- [ ] Keep the implementation within the repository's file and function size
      limits, decomposing responsibility-specific code where necessary.
- [ ] Add focused compiler regressions rather than one opaque end-to-end test.
- [ ] Preserve the one-way lexer -> parser -> AST -> semantic -> codegen
      pipeline.
- [ ] Confirm no project-specific names, AIE logic, or external application
      sources were introduced.
- [ ] Run:
      `cargo fmt --all -- --check`
- [ ] Run:
      `cargo check --all-targets --all-features`
- [ ] Run:
      `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run:
      `cargo test --all-targets --all-features`
- [ ] Run `scripts/check_source_limits.sh` and `git diff --check`.

### Gate 31.6 evidence

- All required checks pass from a clean working tree state.
- Regression evidence covers semantic rejection, object emission, executable
  execution, deterministic symbols, and generic specialization.

## Gate 31.7: Completion and documentation

- [ ] Record exact command evidence for every completed gate.
- [ ] Update ADR-0058 with implementation evidence and approved deviations.
- [ ] Update the compiler architecture documentation and language guide so
      they describe the actual constant visibility and native behavior.
- [ ] Mark Phase 31 complete only after every applicable acceptance criterion
      has direct repository evidence.

## Non-goals

- No runtime mutable global configuration is introduced.
- No new import syntax or alternate facade mechanism is introduced.
- No constant is duplicated into a consumer module or hardcoded as an
  application workaround.
- No AIE, tokenizer, encoder, or other external-project implementation is
  added to this phase.

## Completion criteria

Phase 31 is complete only when an exported constant reached through a
canonical parent facade is correctly available in nested native modules and
generic specializations, while private and bypassed declarations remain
rejected and strict, object, executable, tooling, determinism, and repository
quality evidence all pass.
