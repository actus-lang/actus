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
`case` branch. The source-level types are identical, and both semantic analysis
and native lowering must preserve the branch value through the canonical enum
layout and ABI.

- [x] Add a minimal compiler-only reproducer for the nested `Result` return
      mismatch with a non-primitive success payload.
- [x] Trace and repair semantic result validation and native case lowering for
      final expressions in block branches; the original identical-type
      diagnostic was caused by a branch value being classified as absent.
- [x] Add native executable regressions for both `Ok` and `Err` branches and
      for a valid nested `Option`/`Result` chain.
- [x] Verify that the fix preserves discriminant width, payload address,
      ownership cleanup, and the existing invalid-discriminant trap path.
- [x] Run the full compiler quality suite and the Twin-e strict regression
      suite with the repaired compiler.
- [x] Update ADR-0069 and this roadmap with exact commands, diagnostics, and
      current commit evidence.

Current evidence:

- Compiler `cargo check --all-targets --all-features`: passed.
- Compiler application suite: 33 passed.
- Twin-e strict suite with rebuilt compiler: 27 passed.
- Full Actus suite: passed across all targets and features.
- Clippy with `-D warnings`: passed.
- Source-limit script and `git diff --check`: passed.
- Installed compiler strict Twin-e executable: passed with no floating-point
  instructions in generated native IR.
- The loop-target failure was repaired by preserving loop targets through
  nested short-circuit operation lowering and case branch tails.
- Generic statement cases and value-producing case expressions now use
  separate type and lowering paths, so ordinary statement branches do not
  require a value.

### Gate 36.7 — Const-generic aggregate payloads in enum cases

This gate extends native enum support to the composition already required by
the language: a bounded const-generic array used as the payload of
`Result[T, E]` or `Option[T]`, including case binding and native return paths.

- [x] Reproduce `Result[Array[u8, 20], Failure]` failing at a case payload
      binding with `E1023 unknown type Array[u8,20]`.
- [x] Preserve the complete nested `TypeName` when binding enum payload
      patterns instead of reducing it to a plain name string.
- [x] Validate nested payload type applications through the normal semantic
      type resolver.
- [x] Resolve array size and alignment while specialized enum layouts are
      being constructed.
- [x] Add a native executable regression that constructs, forwards, matches,
      and indexes a const-generic array payload.
- [x] Run the complete Actus quality suite and reinstall the compiler before
      retesting Twin-e.
- [x] Record the installed compiler evidence and close the gate only after
      Twin-e strict validation remains green.

The implementation and contract are recorded in ADR-0070.

Gate 36.7 compiler evidence:

- `cargo test --all-targets --all-features`: passed, including 34 application
  tests and the new const-generic enum payload regression.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `scripts/check_source_limits.sh`: passed.
- `git diff --check`: passed.
- `cargo install --path . --force --locked`: installed the corrected compiler.
- Installed binary SHA-256: `f33ea7dba571b8704a377d832d4d690a4a621f6802a90b1760f9b2688a284ead`.
- Installed compiler digest after the final native aggregate fix:
  `f8f3a659e3b8a589018a833559ead567b2ad5dd0742192b656c1bf5d69dc0430`.
- Twin-e `actus test --strict`: 34 passed, 0 failed.

### Gate 36.8 — Classify the downstream native trap before changing lowering

This gate records the native failure first reported by Twin-e's hardware-neutral
action routing. The initial assumption was an enum-valued `Result` lowering
defect. A strict disposable Twin-e executable reproduced the failure, but the
native backtrace and disassembly localized it to the application expression
`observation_key(...) as u32` inside `observation_token_id`. The `u64` hash is
not bounded to `u32`; Actus's checked cast intentionally emits `ud2` when the
value does not fit. The compiler must not be changed to suppress that valid
overflow trap.

- [x] Reproduce the failure in a disposable Twin-e copy without modifying
      either repository.
- [x] Confirm strict native build and zero-float verification before execution.
- [x] Capture the native result: exit status `132` (`SIGILL`) and backtrace in
      `verb_observation_token_id`/`verb_observation_key`.
- [x] Inspect the generated instruction and identify the checked `u64` to
      `u32` narrowing guard immediately before `ud2`.
- [x] Rule out a compiler enum-discriminant defect with direct
      `Result[Enum, Enum]`, nested-facade, and enum-field compiler fixtures.
- [x] Record that no Actus compiler implementation change is justified by this
      reproduction; the required fix belongs to Twin-e's token identity design.
- [x] Update Twin-e so observation token identity remains bounded without an
      unchecked cast, then rerun its strict action-routing tests.
- [x] Reinstall this compiler only after the downstream fix and record the
      compiler digest and complete validation results.

Gate 36.8 evidence:

- Disposable Twin-e direct mapping probe: exit `41`.
- Disposable Twin-e ActionEvent validation probe: exit `2`.
- Disposable Twin-e full route probe: strict build passed with zero-float
  verification, then exited `132`.
- GDB backtrace stopped in `verb_observation_token_id` after calling
  `verb_observation_key`.
- Object disassembly showed the checked narrowing branch and `ud2`; the hash
  value was not a valid `u32`.
- Compiler-only enum and nested-facade fixtures passed, so no compiler source
  change was made for this report.

The downstream bounded identity fix and the installed compiler regression
suite are now complete; the strict Twin-e suite passes with 34 tests.

### Gate 36.9 — Native lowering of const-generic aggregate parameters

This gate tracks a separate native backend limitation exposed by Twin-e's
host scale validation. A valid source fixture using concrete calls to
`CorticalFabric[64]`, `CorticalFabric[256]`, and `CorticalFabric[1024]` passes
strict semantic checking, but executable emission currently fails with:

```text
native backend cannot lower parameter `fabric` of type `CorticalFabric`
```

The compiler must support this composition as a first-class native contract;
the downstream project must not split one logically generic benchmark into
duplicated size-specific implementations merely to avoid the limitation.

- [x] Add a compiler-only reproducer that defines a bounded aggregate with a
      const-generic array field and passes the specialized aggregate through
      an `ins` or `abs` native verb parameter.
- [x] Trace the specialized aggregate's layout, parameter ABI, ownership role,
      and native declaration from const-generic resolution through Cranelift
      lowering.
- [x] Lower concrete `CorticalFabric[N]` parameters without erasing the
      const argument, changing the aggregate layout, or weakening ownership
      validation.
- [x] Add accepted native execution regressions for at least three distinct
      const sizes, including a bounded array-backed aggregate larger than the
      current 64-column fixture.
- [x] Add rejected coverage for unsupported or unbounded aggregate parameter
      forms and preserve the existing diagnostic boundary.
- [x] Verify that generic calls, direct concrete calls, facade imports, and
      nested generic calls share the same specialized parameter ABI.
- [x] Run the complete compiler quality suite and record native execution,
      source-limit, and `git diff --check` evidence.
- [x] Reinstall the compiler and rerun Twin-e's scale fixture without a
      source-level duplication workaround; record the exact compiler digest and
      scale output in the downstream roadmap.

Gate 36.9 evidence:

- The compiler-only `ins` regression passes for `Fabric[64]`, `Fabric[256]`,
  and `Fabric[1024]`; the direct concrete `Fabric[256]` call also passes.
- The compiler-only `abs` regression passes for `Fabric[256]`.
- The nested-facade pack-backed aggregate regression passes.
- `cargo fmt --all -- --check`, `cargo check --all-targets --all-features`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --all-targets --all-features`,
  `scripts/check_source_limits.sh`, and `git diff --check` pass.
- The corrected compiler was installed with `cargo install --path . --force
  --locked`; installed binary SHA-256 is
  `f8f3a659e3b8a589018a833559ead567b2ad5dd0742192b656c1bf5d69dc0430`.
- Twin-e's original generic scale fixture builds and runs with strict native
  IR verification and no source-level size-specific workaround:
  `Fabric[64]` 453231 ns, `Fabric[256]` 458686 ns, and `Fabric[1024]`
  459999 ns for 1000 wavefront steps on the documented host.
- Twin-e `actus test --strict`: 34 passed, 0 failed.

Gate 36.9 is complete for the hosted native contract. Embedded target
measurements remain governed by Twin-e Phase 5.6 and are not inferred from
this host evidence.

## Definition of done

Phase 36.9 is complete for the documented hosted native contracts: nested enum
discriminants, const-generic aggregate payloads, and pack-backed aggregate
parameters execute with stable identities and ownership-preserving ABIs. The
invalid-state path remains fail-closed, all required compiler checks pass, and
the implementation and evidence are documented in focused reviewable changes.
