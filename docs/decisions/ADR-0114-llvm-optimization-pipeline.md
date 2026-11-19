# ADR-0114: LLVM Optimization Pipeline

## Status

Proposed.

## Context

The LLVM backend lowers a program to IR, verifies it, and hands it to the
target machine. It never runs the LLVM IR optimizer: the configured
optimization level reaches only the Cranelift ISA, and the LLVM target
machine is created at `CodeGenLevelNone` for every profile. A release build
is therefore an unoptimized build, two to four times slower than the same
algorithm compiled by a C compiler.

The IR is also not ready for the optimizer:

- Temporaries for aggregates, enums, call results, return slots and scalar
  loans are `alloca`s emitted where the value is built. Inside a loop each
  iteration allocates new stack and never releases it. A loop that builds a
  struct fifty million times overflows an 8 MiB stack on the LLVM backend
  while the Cranelift build of the same program returns its result. This is
  a correctness defect independent of optimization.
- An `alloca` outside the entry block is a dynamic allocation. `mem2reg` and
  SROA promote only static allocas, so every such value stays in memory even
  when the optimizer runs.
- The zero-divisor guard, the arena capacity guard and the fallback block of
  an exhaustive `case` ended in `unreachable`. LLVM treats reaching
  `unreachable` as undefined behavior: unoptimized, `x % 0` ran into the next
  block and crashed; optimized, the guard was deleted and `x % 0` returned a
  value.

The language makes promises the optimizer must keep: checked arithmetic,
bounds and failure traps, volatile access through `VolatileAddress`,
zero-on-drop storage, deterministic objects, and stack-frame evidence for
freestanding targets.

## Decision

### Profiles

A build profile selects one optimization level, and the level selects both
the LLVM IR pipeline and the target machine code generation level.

| Profile            | `opt_level` | IR pipeline   | Code generation |
|--------------------|-------------|---------------|-----------------|
| `debug` (dev)      | `none`      | none          | `None`          |
| `release`          | `speed`     | `default<O3>` | `Aggressive`    |
| `size`             | `size`      | `default<O2>` with `minsize` | `Default` |

`opt_level` gains the value `size`, and the manifest gains a `[profile.size]`
table; `--profile size` selects it. A profile table may override its level
as today. Cranelift maps `size` to its `speed_and_size` setting. The
pipeline runs through the new pass manager (`LLVMRunPasses`) with the target
machine attached, so cost models and the data layout match the target.

### IR ready for promotion

Every `alloca` is emitted in the entry block of its function, before any
other instruction, whatever block requests it. Lowering requests a slot
through one function that positions a dedicated entry builder; no other
code calls `LLVMBuildAlloca`. Slots are static, so a loop reuses one slot
and `mem2reg` and SROA can promote it. This applies at every level,
including `none`.

### Safety contract under optimization

The optimizer may remove only what the language does not observe:

- Integer arithmetic is lowered without `nsw`, `nuw` or `exact` flags, so
  the optimizer never treats a wrapped result as undefined behavior. This
  ADR keeps today's arithmetic semantics; whether `+`, `-` and `*` trap on
  overflow is a separate language decision.
- Every guard (division and remainder by zero, `MIN / -1`, shift count,
  array index, checked cast, arena capacity) and every block lowering marks
  as impossible ends in a call to `llvm.trap`, never in a bare
  `unreachable`. The optimizer may delete a guard only by proving its
  condition false, which is the same guarantee the program had.
- Volatile reads and writes and zero-on-drop go through runtime bridges in
  the runtime archive, which the optimizer cannot see into; the program
  object is optimized alone, without link-time optimization against the
  runtime.
- No `noalias`, `nonnull` or `dereferenceable` attribute is emitted unless
  the ownership role proves it; adding such attributes is a separate
  decision with its own tests.

A differential test builds each fixture of `tests/fixtures/optimization` at
every profile on both backends and requires the same exit status. Each guard
kind has a fixture that must end through the trap instruction (`SIGILL` or
`SIGTRAP`) at every level; any other signal fails it.

### Determinism

Objects stay byte-identical for the same source, target and profile. The
pipeline is single-threaded and its pass string is fixed per level. Unit
metadata (`.actmeta`, format 6) records the resolved optimization level,
because a profile table can change the level of a profile; an artifact built
at another level is stale.

### Stack-frame evidence

Stack-frame evidence fixtures pin the `Default` level, which runs code
generation without the IR pipeline, so their frame sizes do not move with
the pipeline. Frame evidence for optimized builds is a separate decision.

### Performance gate

`benchmarks/kernels` holds small kernels (branchy integer loops, recursion,
a sieve over an array, shifts, a struct built and borrowed in a loop), each
in Actus and in C. `scripts/benchmark_gate.py` builds the Actus kernel with
the `release` profile on the LLVM backend and the C twin with `clang -O3`,
which shares the optimizer, so the gate measures what Actus hands LLVM
rather than a difference between compilers. It counts instructions under
Callgrind from `main`, which is deterministic on shared runners where wall
time is not. A kernel fails when its exit status differs from C or when
Actus executes more than 1.25 times the instructions of C; the threshold
lives in `benchmarks/kernels/gate.toml`.

## Consequences

- Release builds run the LLVM optimizer for the first time; compile time at
  `speed` and `size` grows, `none` stays unchanged.
- The stack overflow of temporaries in loops and the undefined behavior of
  the zero-divisor, arena and `case` fallback guards are fixed on the LLVM
  backend at every level. A freestanding object no longer needs the
  `__chkstk` stack probe that dynamic allocas required.
- `[profile.size]`, `opt_level = "size"` and `--profile size` are new
  manifest and CLI surface.
- Unit metadata gains the optimization level; older `.actmeta` files are
  stale and rebuilt.
- The LLVM-only CI job runs the Callgrind gate; the differential matrix runs
  with the full test suite.
