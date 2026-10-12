# ADR-0080: Backend-Neutral Generic Specialization

- **Status:** Accepted
- **Date:** 2026-10-12
- **Scope:** Generic type and verb specialization used by native backend emission

## Decision

Generic AST specialization is owned by `src/codegen/generic/`. It materializes
concrete generic declarations, substitutes generic types in declarations and
bodies, expands transitive instances, and derives deterministic specialized
verb names.

The specialization layer may use Actus AST and semantic generic contracts. It
must not import Cranelift, LLVM, target handles, object emitters, or runtime
providers. Backend-specific layout registries and machine emission remain in
their respective backend directories.

The CLI uses the same specialization functions for Cranelift and LLVM. A
requested LLVM build therefore does not depend on the Cranelift feature merely
to materialize generic declarations.

## Invariants

- Specialization preserves source declaration order for non-generic items.
- Every materialized declaration has no remaining generic parameters.
- Specialized names are deterministic and collision-safe.
- Type substitution does not change ownership roles, call-site roles, or
  source spans used for diagnostics.
- Backend selection occurs after specialization and never changes its result.

## Boundaries

Generic layout calculation that depends on a machine backend remains owned by
that backend. Moving specialization does not claim complete generic lowering,
LLVM parity, or support for every generic runtime shape.

## Verification

The shared specialization module compiles with the default Cranelift feature
set and with `--no-default-features --features llvm --lib`. Existing CLI,
LLVM backend, and architecture-boundary tests continue to pass with both
backends enabled.
