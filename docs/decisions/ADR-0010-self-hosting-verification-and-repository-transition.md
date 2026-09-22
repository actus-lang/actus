# ADR-0010: Self-Hosting Verification and Repository Transition Criteria

- Status: Accepted future transition policy
- Date: 2026-09-22
- Scope: Actus compiler bootstrap and repository architecture

## Context

Actus is currently bootstrapped in a single Rust repository. Splitting the
repository into independently compiled Actus units before the language and
build system can support that split would create architectural risk and make
the bootstrap path harder to verify.

The transition must therefore be governed by observable self-hosting results,
not by repository size or the number of compiler modules.

## Decision

The current Rust monolith remains intact until the self-hosting verification
checklist is complete. No early repository decomposition is permitted.

Self-hosting is verified through a bootstrap fixed point:

```text
Stage 0: Rust compiler builds the Actus compiler.
Stage 1: Rust compiler builds the Actus compiler written in Actus.
Stage 2: Actus compiler builds that Actus compiler.
Stage 3: The Actus-built compiler builds itself repeatedly.
```

Stage 2 and Stage 3 are the objective criteria for an independent Actus
compiler. A successful Stage 0 or Stage 1 is bootstrap progress, not
self-hosting completion.

## Required Verification Conditions

Repository transition is permitted only when all conditions below are met.

### Language and Compiler Foundation

- The Actus compiler source is implemented sufficiently in Actus to build a
  complete compiler.
- Lexer, parser, AST, semantic analysis, ownership, borrowing, cleanup,
  diagnostics, formatter, native code generation, and FFI contracts are
  covered by tests.
- The Actus compiler can produce a native executable from its own source.

### Bootstrap Fixed Point

- Stage 2 completes without relying on an unreproducible manual step.
- Stage 3 completes repeatedly from a clean build directory.
- Repeated builds produce equivalent compiler interfaces and deterministic
  diagnostics.
- Generated artifacts are reproducible under the same toolchain, target,
  profile, and source inputs.

### Package and Unit Infrastructure

- `package`, `module`, and `unit` boundaries are implemented.
- `::` imports, `open` visibility, and module resolution are implemented.
- `Arca.toml` declares packages and units.
- `Arca.lock` records exact dependency resolution.
- Unit interface metadata such as `.actmeta` is generated and validated.
- Unit cache invalidation rejects incompatible toolchains and targets.
- Dependency cycles are rejected deterministically.

### Standard Library and Build Profiles

- `core`, `alloc`, and `std` exist as separate library layers.
- Their dependency direction is enforced as `core -> alloc -> std`.
- Freestanding and hosted profiles are implemented and tested.
- Standard-library implementations are not embedded in the compiler binary.

### Repository and Toolchain Safety

- The full test suite passes for both the Rust bootstrap compiler and the
  Actus-built compiler where applicable.
- CI can build and test the bootstrap path from a clean checkout.
- The Rust compiler remains available as a documented fallback until the
  Actus compiler passes the fixed-point verification.
- Documentation, manifests, lockfiles, and build scripts describe the same
  transition state.

## Transition Sequence

Once the checklist is complete, migration proceeds incrementally:

1. Introduce Actus compiler units while preserving the Rust bootstrap path.
2. Build those units through the verified Actus compiler.
3. Move standard-library layers into independently managed Actus units.
4. Move Arca build integration to the Actus unit graph.
5. Retain the Rust compiler as a temporary fallback during a documented
   deprecation period.
6. Remove or archive the Rust bootstrap only after an additional release
   cycle confirms the self-hosted path.

The migration must not be performed as one irreversible repository rewrite.
Every step requires a passing bootstrap build and a rollback path.

## Non-Goals

This decision does not require the current compiler to be split now. It does
not define target-specific hardware projects, drivers, or SDK structure.
Those concerns remain outside the Actus language repository architecture.

## Consequences

The current Rust monolith remains a stable and inspectable bootstrap base.
Repository decomposition becomes an evidence-based consequence of a working
self-hosted compiler rather than a speculative architectural exercise.

