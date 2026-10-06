# Phase 35: Canonical Generic Result Identity in Native Emission

**Status: Planned**

## Purpose

This phase makes generic result types retain one canonical compiler identity
across semantic analysis, nested facade propagation, native dependency planning,
and native emission.

The motivating failure has this shape:

```text
error[E1026]: return type mismatch
expected `Result[Int, IoError]`, found `Result[Int, IoError]`
```

The rendered type names are identical, but the native emitter treats the two
values as different types. The failure is especially visible when a public verb
returns `Result[T, E]`, calls another public verb through a nested facade, and
returns or pattern-matches that result at an executable call site. Strict
semantic checking can therefore succeed while native executable emission fails.

This is a compiler correctness issue. It must be solved in the compiler's type
identity and native planning layers, not by duplicating implementations,
weakening ownership roles, adding facade bypasses, or changing application
source to avoid `Result`.

## Scope

- Generic type identity for `Result[T, E]` and the same identity rule for all
  parameterized nominal types.
- Identity propagation through parent and nested public facades.
- Native lowering, dependency collection, and executable emission.
- Ownership and `case dat` paths that consume a generic result.
- Diagnostics that distinguish a real mismatch from an internal identity split.
- Regression coverage at semantic, object, executable, and nested-facade levels.

## Non-goals

- No change to Actus ownership roles or call-site syntax.
- No change to `Result` layout, enum representation, ABI, or standard-library
  API.
- No implicit coercion between different generic instantiations.
- No application-specific compiler special case for `Result[Int, IoError]`.
- No facade bypass or duplicated native symbol as a workaround.
- No change to serialized formats or runtime filesystem behavior.

## Failure contract

For a canonical generic instantiation, all of the following must identify the
same type:

```text
Result[Int, IoError]
```

- the semantic model's declared return type;
- the return type inferred for a nested facade call;
- the type of a local binding receiving that call;
- the type consumed by `case dat` deconstruction;
- the native dependency graph node;
- the object-level ABI signature;
- the executable lowering signature.

A different generic argument list remains a different type. For example,
`Result[Int, IoError]` and `Result[u32, IoError]` must not unify silently.

## Gates

### Production implementation policy

The implementation must establish one compiler-owned identity contract before
any semantic, facade, cache, or native lowering change is accepted.

- [ ] Define one structured `TypeIdentity` representation for nominal generic
      types, ordered type arguments, const arguments, ABI details, and layout
      details.
- [ ] Route semantic analysis, facade export propagation, generic caching,
      specialization, dependency planning, and native lowering through that
      representation.
- [ ] Keep source spans, facade paths, call-site locations, display strings,
      and diagnostics metadata outside structural identity.
- [ ] Add permanent test-only identity snapshots and invariant assertions at
      each pipeline boundary. Do not add unconditional debug printing or an
      environment-only production behavior.
- [ ] Preserve public visibility, ownership roles, native ABI, symbol
      stability, and serialized compatibility while changing identity logic.
- [ ] Do not close a gate with duplicated application code, source rewrites,
      `meta limitless`, weakened validation, or a facade bypass.

### Gate 35.0 — Reproducer admission and compiler baseline

This prerequisite prevents a consumer ownership or syntax error from being
mistaken for a generic identity failure.

- [x] Start from the synchronized compiler `main` revision used for the
      investigation.
- [x] Confirm the compiler-only fixture has valid ownership roles and reaches
      native emission without an earlier semantic diagnostic.
- [x] Run the focused application baseline and record its result before
      changing semantic or native identity code.
- [x] Keep the fixture generic and independent of any application repository,
      domain vocabulary, or external filesystem layout.

#### Evidence — 2026-10-06

- Compiler baseline: merge commit `c72c9d5`.
- A generic `Result[Int, IoError]` fixture with nested public facades,
  separate producer/flush/save modules, `dat` result deconstruction, and a
  mutable state parameter passes strict semantic checking and reaches native
  emission without an earlier semantic diagnostic.
- The Actus application test baseline passes: 25 tests, 0 failures.

### Gate 35.1 — Reproduce and localize the identity split

- [x] Add a minimal generic `Result[T, E]` source fixture that passes semantic
      checking and fails native emission with the identical-name mismatch.
- [x] Add the same fixture through one nested public facade and through a
      direct module import for comparison.
- [x] Record the compiler revision, command, complete diagnostic, and source
      span for each failure.
- [x] Identify whether the split occurs during generic specialization, facade
      export propagation, native dependency collection, or lowering.
- [x] Add an internal debug assertion or test-only identity trace that shows
      the canonical type key at each pipeline boundary.

#### Investigation note — 2026-10-06

- The corrected compiler test fixture
  `generic_result_nested_facade_baseline_builds_natively` passes strict
  checking and native executable emission.
- The same fixture also passes through
  `generic_result_direct_child_facade_baseline_builds_natively`, using the
  direct `feature::runtime` public facade path. Both paths produce native
  executables with the same `Result[Int,IoError]` contract.
- An earlier draft produced `E1026` because a value-producing `case` block
  ended with `Ok(bytes);` instead of `return Ok(bytes);`. The diagnostic span
  identified a fixture contract error, so that draft is not accepted as a
  generic identity reproducer.
- A detached worktree at compiler revision `b09c65c` reproduced the historical
  failure by restoring the pre-fix role-free native enum name. The command was
  `cargo test --test applications
  generic_result_nested_facade_baseline_builds_natively -- --nocapture`.
  The complete diagnostic was `native backend cannot lower return type
  Option`; the harness reported it at `tests/applications.rs:76` because the
  backend diagnostic did not carry an Actus source span. Semantic checking had
  already succeeded.
- The split was localized to generic specialization/native lowering: the
  structural key `Result[Item]` was incorrectly used as the native definition
  name where the ABI-aware name `Result[abs Item]` was required. Facade export
  propagation was not the source of the mismatch.
- The permanent test-only trace
  `specialization_trace_separates_structural_and_abi_identity` asserts both
  identities at the specialization boundary. Together with the direct and
  nested facade native regressions, Gate 35.1 is closed.

### Gate 35.2 — Define the canonical generic identity contract

- [ ] Document the canonical key for nominal generic types, including the
      declaration identity, ordered type arguments, const arguments, and
      relevant ABI or layout parameters.
- [ ] Define which identity data is structural and which data is source-site
      metadata only.
- [ ] Ensure source spans, facade paths, and call-site locations never create a
      second semantic type identity.
- [ ] Define cache ownership and invalidation rules for generic instances.
- [ ] Record the accepted design in an ADR under `docs/decisions/`.

### Gate 35.3 — Repair semantic and facade propagation

- [ ] Make nested facade export resolution reuse the canonical generic type
      identity rather than reconstructing a display-equivalent type.
- [ ] Preserve private declarations and facade visibility rules.
- [ ] Preserve explicit `erg`, `abs`, `dat`, and `ins` validation at every call
      site.
- [ ] Add accepted tests for direct and nested-facade generic return values.
- [ ] Add rejected tests proving different generic arguments remain distinct.
- [ ] Verify that generic cache entries cannot overwrite one another when the
      same specialization is reached through different facade paths.

### Gate 35.4 — Repair native dependency planning and lowering

- [ ] Use the canonical generic identity as the native dependency key.
- [ ] Ensure one generic specialization produces one stable native type and
      one compatible dependency node per ABI contract.
- [ ] Preserve all transitive dependencies when the specialization is reached
      through a nested facade.
- [ ] Make object emission and executable emission use the same resolved type
      identity.
- [ ] Add diagnostics for an actual ABI mismatch instead of reporting a false
      same-name return mismatch.
- [ ] Verify that native emission does not introduce duplicate symbols,
      duplicate enum layouts, or unsafe casts.

#### Implementation note — 2026-10-06

- Structural `TypeIdentity` selects and reuses generic instances across
  semantic, cache, layout, specialization, and dependency boundaries.
- ABI-aware canonical names remain the native definition names. They retain
  ownership-role information such as `Option[abs PathComponent]`, because
  native layout and ABI contracts must not collapse that information into the
  role-free structural cache identity.
- External native symbol bindings are scoped by module namespace and source
  declaration. A unique-source fallback preserves compatibility for the
  existing single-module import path without merging distinct declarations.
- The focused direct and nested facade native tests pass, and the complete
  applications suite passes with 25 tests and 0 failures. The gate remains
  open until object-level identity assertions, duplicate-definition checks,
  and the remaining acceptance evidence are complete.

### Gate 35.5 — Ownership, aggregate, and case-deconstruction coverage

- [ ] Accept `case dat result` when the result is returned from a generic verb
      through a nested facade.
- [ ] Cover `Result.Ok` and `Result.Err` paths with owned aggregate payloads.
- [ ] Cover borrowed and exclusive arguments that produce a generic result.
- [ ] Reject use-after-move and invalid ownership roles after deconstruction.
- [ ] Verify branch joins and cleanup remain deterministic for both variants.
- [ ] Cover repeated call sites using one canonical specialization without
      losing source-specific rewrite spans.

### Gate 35.6 — Native and executable acceptance evidence

- [ ] Add semantic, object, and executable regression tests for the minimal
      reproducer.
- [ ] Add a nested-facade regression with at least two call sites sharing one
      generic specialization.
- [ ] Verify strict object and executable builds both succeed.
- [ ] Verify the emitted native representation contains no duplicate generic
      type definitions or incompatible result layouts.
- [ ] Preserve zero-floating-point evidence for integer-only fixtures where the
      fixture requires it.
- [ ] Record exact compiler revision, commands, target profile, and outputs.

### Gate 35.7 — Tooling, diagnostics, and documentation

- [ ] Keep formatter and LSP type display based on the same canonical identity
      used by native emission.
- [ ] Add a deterministic diagnostic for genuine generic identity or ABI
      incompatibility.
- [ ] Prevent the diagnostic renderer from collapsing distinct internal types
      into misleading identical display names without identity context.
- [ ] Update the language guide with the canonical generic identity contract.
- [ ] Update the relevant architecture documentation and acceptance report.
- [ ] Add a migration note if any internal cache key or native symbol naming
      changes.

### Gate 35.8 — Full quality and compatibility gate

- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo check --all-targets --all-features` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --all-targets --all-features` passes.
- [ ] Existing generic call-site, nested-facade, ownership, and native tests
      remain green.
- [ ] No application-specific vocabulary or workaround enters compiler code.
- [ ] The final acceptance report records known limits and unresolved follow-up
      work.

## Evidence policy

A passing semantic check alone does not close this phase. The failure under
investigation occurs after semantic analysis, so acceptance requires native
object and executable evidence. Each closed gate must name the test, command,
compiler revision, and relevant output.

A workaround that changes the application source, duplicates a generic
implementation, bypasses a facade, weakens a role, or adds an undocumented
exception does not count as evidence and must not be used to close a gate.

## Exit criteria

Phase 35 is complete only when all gates are checked, the canonical identity
ADR and language documentation are published, the minimal reproducer passes
semantic and native emission, nested facade regressions pass, and the complete
quality suite is green.
