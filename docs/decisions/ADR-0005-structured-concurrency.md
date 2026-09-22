# ADR-0005: Structured Concurrency and Owned Tasks

- Status: Accepted future design
- Date: 2026-09-22
- Scope: Actus concurrency model

## Context

Actus must support concurrent execution without requiring a garbage collector
or hiding resource lifetime inside an implicit runtime. A background operation
must have an explicit owner and a deterministic completion path.

The concurrency model must also fit Actus's existing `erg`, `abs`, and `dat`
ownership roles.

## Decision

Actus uses structured concurrency by default. The `act` operation always
creates a task and returns an owned `erg Task` handle:

```act
erg task = act(process, dat payload);
join(dat task);
```

The `dat` argument transfers ownership of `payload` into the task. After the
operation begins, the caller cannot use or drop the moved binding. The task
becomes responsible for the payload and deterministically cleans it up when
execution finishes.

A task handle is itself an owned resource. It must be consumed through an
explicit lifecycle operation:

- `join(dat task)` waits for completion and consumes the task handle;
- `detach(dat task)` deliberately transfers responsibility to the scheduler
  for a detached task.

Fire-and-forget execution is therefore never the implicit behavior of `act`.

## Safety Guarantees

The ownership transfer prevents the original owner and the task from
mutating the same moved payload through separate owners. This is a compiler
guarantee for the transferred resource, not a claim that all possible data
races disappear. Shared mutable global state, raw pointers, interior
mutability, and unsafe FFI remain subject to their own safety rules.

Task entry points must accept ownership-compatible arguments. Borrowed `abs`
values must not outlive their lexical scope or escape into an asynchronous
task unless a future lifetime design explicitly permits it.

## Core and Standard Library Boundary

`act` is not provided by the freestanding `core` layer. A hosted environment
lowers it to the standard library task implementation, conceptually:

```text
act(process, dat payload)
    -> std::task::spawn(process, dat payload)
```

The lowering occurs only after parsing and ownership/borrow analysis. On a
bare-metal target, `act` is rejected unless the selected target explicitly
provides a scheduler hook with the required task contract.

The scheduler, task storage, stack policy, failure handling, and cancellation
policy belong to the selected runtime or target. They must not be hardcoded
into the language frontend.

## Deferred Design Questions

The following details remain future design work and must be specified before
concurrency is implemented:

- task result and error propagation;
- task cancellation and shutdown behavior;
- scheduler failure and resource cleanup when spawning fails;
- task stack and allocation policy for hosted and bare-metal targets;
- whether `detach` is permitted in every package profile;
- synchronization primitives for intentionally shared state.

## Non-Goals

This decision does not add concurrency syntax or runtime code to the current
alpha compiler. It defines the permanent architectural direction so that
future task support does not introduce implicit ownership, garbage collection,
or untracked background execution.

