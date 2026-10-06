# Phase 36: Native Enum Discriminant Stability Across Nested Generic Calls

## Purpose

Repair native lowering when `Option[T]` and `Result[T, E]` values cross nested
generic calls. A semantically valid enum value must preserve its discriminant
and payload ABI from the callee through every generic specialization and into
the caller's `case` expression.

The phase is driven by a reproducible Twin-e failure: strict checking and the
existing test interpreter accept resident neurogenesis, but a strict native
executable reaches compiler-generated `ud2` in a valid `Option` or `Result`
branch. The fix belongs in Actus native lowering and must remain independent of
Twin-e-specific names or layouts.

## Scope and invariants

- Preserve the existing enum representation and public Actus syntax.
- Preserve ownership roles, cleanup behavior, generic type identity, and native
  ABI contracts unless a reviewed ABI correction is required.
- Keep `Option[T]` and `Result[T, E]` discriminants explicit and stable across
  direct calls, nested generic calls, facade calls, and returned payloads.
- Keep invalid discriminants fail-closed through the compiler's existing invalid
  path. A valid discriminant must never reach a compiler-generated `ud2`.
- Do not add application-specific special cases, facade bypasses, duplicated
  implementations, or source-level workarounds.
- Update the language documentation and ADR only for behavior actually verified
  by compiler tests.

## Gates

### Gate 36.1 — Reproduce and localize the native failure

- [x] Add a minimal compiler fixture that passes strict semantic checking and
      reaches native emission with a nested generic `Option[T]` return.
- [x] Add a matching `Result[T, E]` fixture covering both `Ok` and `Err` paths.
- [x] Reproduce the failure as an executable test, recording the exact exit
      status and generated native failure location.
- [x] Confirm the failure is independent of Twin-e modules, persistence, and
      filesystem APIs.
- [x] Preserve a regression fixture that fails before the implementation fix.

### Gate 36.2 — Trace discriminant and payload ABI contracts

- [x] Identify the canonical enum layout source used by semantic analysis,
      generic specialization, native dependency planning, and code generation.
- [x] Document discriminant width, payload storage, alignment, ownership state,
      and return convention for `Option[T]` and `Result[T, E]`.
- [x] Verify that nested generic specializations use the same ABI-aware enum
      identity at definition, call, return, and case-lowering sites.
- [x] Add assertions or diagnostics at the narrowest compiler boundary where
      an invalid discriminant or incompatible enum ABI would otherwise be
      emitted.
- [x] Record the findings in an ADR before changing the lowering contract.

### Gate 36.3 — Fix native enum return and case lowering

- [x] Preserve the discriminant when an enum is returned from a generic callee.
- [x] Preserve the payload address/value and ownership state across the return
      ABI for scalar, aggregate, and nested generic payloads.
- [x] Lower `case` branches using the canonical discriminant representation.
- [x] Keep invalid discriminants fail-closed without treating valid states as
      unreachable.
- [x] Keep cleanup and move/drop behavior correct on every branch.
- [x] Avoid changes to unrelated enum, facade, or generic identity behavior.

### Gate 36.4 — Native regression coverage

- [x] Add native executable tests for direct and nested `Option[T]` returns.
- [x] Add native executable tests for direct and nested `Result[T, E]` returns.
- [x] Cover success and failure branches with scalar payloads.
- [x] Cover aggregate payloads and ownership transfer where supported.
- [x] Cover nested facade import paths without bypassing the parent facade.
- [x] Assert successful process exit and expected branch results; assert no
      `SIGILL` or compiler-generated trap is reached.

### Gate 36.5 — Full compiler validation and documentation

- [x] Run `cargo fmt --all -- --check`.
- [x] Run `cargo check --all-targets --all-features`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo test --all-targets --all-features`.
- [x] Run `scripts/check_source_limits.sh` and `git diff --check`.
- [x] Update the ADR, roadmap evidence, and relevant native-lowering
      documentation with exact commands and results.
- [x] Install and verify the fixed compiler in the Twin-e environment, then
      rerun the resident probe and confirm the original native path completes.
- [x] Add aggregate-payload coverage, explicit invalid-discriminant assertions,
      and close the phase after those final acceptance items.
- [x] Close the phase only after all gates are checked and no unexplained
      native failure remains.

## Evidence recorded 2026-10-06

- `cargo fmt --all -- --check` passed.
- `cargo check --all-targets --all-features` passed.
- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `cargo test --all-targets --all-features` passed.
- `scripts/check_source_limits.sh` and `git diff --check` passed.
- The fixed compiler was installed with `cargo install --path . --force
  --locked`.
- Twin-e `actus test --strict` passed with 18 tests.
- Twin-e `benchmarks/aie_resident_api_probe.act` built in strict mode, passed
  zero-float native IR verification, and exited successfully after printing
  `AIE_RESIDENT_PROBE complete`.
- `nested_generic_result_preserves_aggregate_payload_ownership_natively` and
  `nested_facade_generic_result_preserves_aggregate_payload_natively` both
  passed with exit code 41.
- Existing semantic exhaustive-pattern validation and native trap regressions
  confirm that invalid enum states remain fail-closed while valid states use
  the merge path.

The original Phase 36 acceptance gates are complete. Gate 36.6 is an active
follow-up acceptance extension for a newly reproduced nested `Result` lowering
identity failure.

### Gate 36.6 — Nested `Result` identity through value-producing case branches

This follow-up gate covers a valid native program that validates one
`Result[u64, E]`, then returns `Result[Aggregate, E]` from a value-producing
`case` branch. The source-level types are identical, but native lowering must
also resolve one canonical enum layout identity and ABI for every branch.

- [ ] Add a minimal compiler-only reproducer for the nested `Result` return
      mismatch with a non-primitive success payload.
- [ ] Add a semantic/codegen diagnostic assertion that records the conflicting
      native enum identities instead of printing identical type names.
- [ ] Trace and repair canonical enum identity reuse across the `case` result
      type, enum constructor, function return declaration, and nested generic
      call site.
- [ ] Add native executable regressions for both `Ok` and `Err` branches and
      for a valid nested `Option`/`Result` chain.
- [ ] Verify that the fix preserves discriminant width, payload address,
      ownership cleanup, and the existing invalid-discriminant trap path.
- [ ] Run the full compiler quality suite and the Twin-e strict regression
      suite with the repaired compiler.
- [ ] Update ADR-0069 and this roadmap with exact commands, diagnostics, and
      commit evidence, then close Phase 36.6 only after all checks are green.

## Definition of done

Phase 36.6 is complete when the new nested `Result` reproducer and the
existing generic fixtures execute natively with one stable enum identity,
all required compiler checks pass, the invalid-state path remains fail-closed,
and the implementation and evidence are documented in a focused reviewable
set of commits.
