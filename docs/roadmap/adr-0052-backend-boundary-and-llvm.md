# ADR-0052: Backend Boundary and Optional LLVM Integration

## Status

Proposed. This ADR records the boundary required before Actus adds a second
native code generation backend. The implementation baseline is the annotated
tag `checkpoint/pre-backend-refactor`.

## Context

Actus currently emits native objects through Cranelift. The compiler pipeline
is already separated into lexer, parser, AST, semantic analysis, codegen,
runtime, and target modules, but the `src/codegen` implementation is not yet a
backend boundary. Cranelift types and builders are used throughout lowering,
expression handling, control flow, aggregate memory operations, ABI emission,
runtime declarations, object emission, linker planning, and target ISA setup.

Adding LLVM directly to those modules would duplicate lowering logic and make
ownership, cleanup, layout, and ABI behavior diverge between backends. The
compiler therefore needs a small Actus-oriented contract between semantic
lowering and machine-specific code generation.

## Current implementation inventory

The current Cranelift dependency surface includes:

- `src/codegen/target.rs`: Cranelift ISA lookup and optimization flags;
- `src/codegen/native/`: native function discovery, object emission, and IR
  audit;
- `src/codegen/native_runtime.rs`: runtime bridge declarations and function
  identifiers;
- `src/codegen/linker.rs`: object linking and platform linker arguments;
- `src/codegen/abi.rs`, `layout/`, `types.rs`, and `structs/`: layout, ABI, and
  aggregate lowering decisions that must be split into shared contracts and
  backend implementations;
- `src/codegen/calls/`, `case/`, `expressions/`, `function_definition/`,
  `lowering/`, `enums/`, `arrays.rs`, `arenas.rs`, `cleanup.rs`, and related
  modules: lowering code that currently constructs Cranelift instructions;
- `src/codegen/model.rs`: native cleanup and unwind models that must remain
  backend-neutral even when their current consumers are Cranelift-based.

The complete source inventory is reproducible with:

```sh
rg -l "cranelift" src tests benches | sort
```

The inventory is intentionally recorded before any move so the migration can
prove that shared modules no longer import backend-specific types.

## Decision

Actus will introduce a backend-neutral lowering contract under `src/codegen`
and isolate concrete implementations below backend-specific modules. The
first migration keeps Cranelift as the default backend and preserves its
observable behavior. LLVM is an optional second backend, enabled through a
Cargo feature such as `llvm`.

The intended shape is:

```text
src/codegen/
├── contract.rs          # backend-neutral operations and metadata
├── target.rs            # target profiles and capabilities
├── cranelift/           # current native implementation
└── llvm/                # optional LLVM implementation
```

The contract represents only Actus requirements: typed values, blocks,
control flow, calls, returns, traps, cleanup actions, aggregate layouts,
runtime imports, ABI metadata, and deterministic symbols. It is not intended
to reproduce LLVM's optimizer, instruction selector, register allocator, or
object writer.

## Mandatory contracts

### Dependency isolation

The default build must not require a system LLVM installation or compile LLVM
bindings. LLVM-specific crates, build scripts, discovery, and diagnostics must
be activated only by the explicit feature. CI must test the default and LLVM
feature configurations independently.

### ABI and target isolation

Target selection and backend selection are separate decisions. A target profile
owns pointer width, byte order, alignment, calling convention, object format,
linker flavor, entry contract, and runtime capability set. ARM32/Thumb support
must encode the selected ARM AAPCS rules, including argument registers,
callee-saved registers, aggregate passing, aggregate returns, and 8-byte stack
alignment.

### Semantic preservation

Ownership roles, cleanup order, typed failures, enum discriminants, aggregate
layouts, Region descriptors, serialization layouts, and runtime bridge symbols
are shared Actus contracts. A backend may reject an unsupported contract but
may not silently reinterpret it.

### Evidence boundary

Host compilation proves host compilation only. ARM32/Thumb acceptance requires
target-specific object, link, and execution evidence. A successful LLVM IR
build is not evidence that a Cortex-M ABI call is safe.

## Migration sequence

1. Freeze the checkpoint and inventory the current dependency surface.
2. Define the minimal backend-neutral contract and its tests.
3. Move Cranelift implementation behind the contract without behavior changes.
4. Validate all existing Cranelift tests, objects, symbols, and benchmarks.
5. Add optional LLVM hosted lowering for the shared contract.
6. Add ARM32/Thumb target profiles and AAPCS validation.
7. Add freestanding object/link/target evidence.

Each step is a separate reviewable change. LLVM integration must not begin by
rewriting the semantic analyzer or by adding a second implementation of Actus
ownership rules.

## Consequences

- Cranelift remains usable without LLVM installed.
- The first refactor may move many files without changing generated behavior.
- Backend-neutral contracts add an explicit maintenance boundary.
- LLVM dependency and build costs are isolated behind an opt-in feature.
- ARM32/Thumb support becomes possible, but it requires a separate ABI and
  target validation effort.

## Acceptance evidence

- [x] `checkpoint/pre-backend-refactor` identifies the pre-refactor main
      commit.
- [x] The current Cranelift dependency surface is recorded above.
- [x] The proposed backend-neutral boundary and migration order are recorded.
- [ ] The shared contract is implemented and covered by backend-independent
      tests.
- [ ] Cranelift is isolated without observable behavior changes.
- [ ] Optional LLVM hosted lowering passes the shared contract suite.
- [ ] ARM32/Thumb output passes target-specific ABI and execution evidence.
