# ADR-0082: Shared native lowering and dual backend migration

- **Status:** Accepted
- **Date:** 2026-10-15
- **Scope:** semantic-to-native lowering, Cranelift/LLVM selection, target profiles, and migration compatibility

## Context

Actus originally emitted native objects through Cranelift. Adding LLVM exposed
places where semantic roles, layouts, cleanup, calls, returns, failures, and
runtime symbols were being interpreted independently by a backend. Independent
interpretation made equivalent programs vulnerable to ABI drift and allowed a
backend-specific fallback to hide an incomplete contract.

## Decision

Actus uses one backend-neutral lowering boundary between semantic analysis and
native emission. That boundary owns the supported roles, layouts, cleanup
plans, calls, returns, failures, external symbols, target profile checks, and
typed unsupported-operation diagnostics. Cranelift and LLVM consume the same
neutral module and are responsible only for target-specific object emission.

Backend-specific types must not cross into semantic, AST, or shared lowering
modules. A selected backend is never replaced silently when an operation is
unsupported or when its feature is unavailable. Unsupported shapes fail at the
shared boundary with the source span, backend, target, and failure class.

Cranelift remains the default when its feature is enabled because it is the
original recoverable native path. LLVM is an explicit opt-in feature and is
available independently of Cranelift. Both paths are accepted only for the
source and target contracts covered by parity evidence.

Hosted and freestanding targets remain separate runtime contracts. Hosted
builds may use the configured host provider. Freestanding builds require
explicit startup, linker, and runtime-provider contracts; they may not receive
implicit host services or hidden allocation. The large-aggregate target policy
in ADR-0081 is enforced before either backend emits code.

## Migration impact

- Actus source syntax and ownership roles are unchanged.
- Existing Cranelift programs retain a Cranelift selection and remain
  recoverable from commit `b886bbc66731675e660d10d9d4ae99cd8e5f1524`.
- Native artifact metadata records backend and target identity; changing either
  invalidates stale artifacts and requires a rebuild.
- LLVM requires the `llvm` Cargo feature and a matching LLVM 23.x installation;
  Cranelift requires its Cargo feature and no system LLVM installation.
- Existing backend-specific tests remain useful emitter tests, while shared
  parity tests are required for behavior claims.
- No serialized state layout or standard-library ownership contract changes as
  a consequence of backend selection.

## Evidence and limits

The acceptance matrix includes all-target/all-feature Rust tests, strict Actus
tests, Cranelift-only and LLVM-only builds, hosted executable parity,
freestanding object parity, target-aware negative tests, source/architecture
checks, dependency/license checks, and coverage. Host benchmark results are
recorded separately from freestanding object evidence. This ADR does not claim
embedded hardware execution or universal support for every target triple and
source shape.
