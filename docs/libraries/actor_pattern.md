# Actus Actor Pattern Library

Status: Planned official library design

This document specifies the future `std::actor` library. It is a library
architecture, not a language extension. `actor` is not an Actus keyword, AST
node, compiler primitive, or hidden runtime facility.

## Scope

`std::actor` will provide an actor-style concurrency model using existing
Actus capabilities:

- `role` contracts and `perform` implementations;
- `dat` ownership transfer;
- `erg`, `abs`, and deterministic cleanup rules;
- bounded ring buffers and explicit allocation policies;
- existing structured task and lifecycle primitives where available.

The library must not require changes to the core language grammar. Its public
API will be expressed using ordinary Actus declarations, roles, enums,
structs, and verbs.

## Architectural Model

An actor is an owned, isolated state machine that receives owned messages
through an explicit mailbox and never shares mutable state with another actor.
The library may expose constructors, mailbox handles, message operations, and
lifecycle verbs, but their implementation remains library and target-runtime
code.

The word `actor` may be used as a library type or namespace identifier when
the normal visibility rules permit it. It does not receive special grammar or
semantic treatment.

## Message Passing

Messages are ordinary Actus values, preferably enums with unit or payload
variants:

```act
enum SensorMessage {
    Read,
    Configure { rate: U32 },
    Shutdown,
}
```

Sending a message transfers ownership:

```act
std::actor::send(dat sensor, dat message);
```

The sender loses ownership, the mailbox temporarily owns the message, and the
handler receives ownership when the message is delivered. A message must not
contain a borrow that outlives the operation which created it.

Ownership transfer does not require a physical copy. Resource handles and
other transferable representations may move without copying when the target
allows it. Small value payloads may be copied as an implementation choice.

## Mailboxes

The initial library design supports bounded mailboxes implemented with ring
buffers. Capacity and overflow behavior must be explicit:

- return a `Result` describing a full mailbox;
- apply a documented blocking policy; or
- apply an explicit drop/overwrite policy.

Silent unbounded allocation is not permitted.

Hosted profiles may provide dynamically allocated mailboxes through the
appropriate standard-library layer. Freestanding profiles must be able to use
statically allocated buffers with known size and alignment.

## Lifecycle and Scheduling

An actor handle is an owned resource. The library must provide an explicit
lifecycle, including a deliberate completion, `join`, or `detach` operation.
An actor must not become an abandoned background task by default.

The actor pattern does not define a scheduler. A target may implement it with
a hosted executor, thread pool, event loop, cooperative scheduler, or
interrupt-driven service. Scheduler choice, stack allocation, and wake-up
mechanisms remain library/runtime and target concerns.

The library must distinguish short-lived structured tasks from long-lived
mailbox-driven services. Existing task primitives may be reused, but actor
behavior must not introduce hidden runtime state into the compiler.

## Roles and Message Protocols

Roles may describe the public message protocol or operations supported by an
actor-like library type. `perform` remains compile-time static dispatch and
monomorphization. It is not a replacement for mailbox delivery and does not
implicitly create a runtime vtable.

If dynamic role dispatch is later used by the library, it must use the
language's explicit `dynamic` model and remain separate from `perform`.

## Safety and Isolation Invariants

The library design is governed by these invariants:

1. Actor state belongs to exactly one actor.
2. Actor communication uses message passing rather than shared mutable state.
3. Shared mutable state between actors is prohibited by the library contract.
4. Message ownership crosses the boundary through `dat` transfer.
5. `abs` borrows cannot be stored in a mailbox or retained in actor state.
6. Mailbox capacity and allocation policy are explicit and bounded.
7. Actor handles are owned resources with deterministic lifecycle behavior.
8. Completion, `join`, and `detach` are explicit operations.
9. An actor is not synonymous with an operating-system thread.
10. `perform` is static contract implementation, not actor runtime dispatch.
11. Scheduling and execution strategy belong to the library/runtime backend.
12. Zero-copy is an allowed optimization; ownership transfer is the semantic
    guarantee.

## Cleanup and Failure

Mailbox contents, actor state, and owned handles must follow Actus deterministic
cleanup rules. On normal completion, remaining owned resources are released in
the specified reverse order. The library must define what happens to queued
messages when an actor stops and must prevent both leaks and double drops.

Actor failure policy is intentionally deferred. Restart, supervision, parent
notification, mailbox draining, and system shutdown require a separate library
design before they become part of the official API.

## Core and Standard-Library Boundaries

`std::actor` is not part of `core`. A freestanding target may provide a
compatible actor library only when it supplies an explicit executor or target
hook. A hosted implementation may use `alloc` and operating-system services.

The compiler must understand only the ordinary Actus constructs used by the
library. It must not add actor-specific parsing, hidden scheduling, implicit
mailboxes, or actor-specific ownership exceptions.

## Future Work

- Define the public `std::actor` types and verbs.
- Define bounded ring-buffer APIs for hosted and freestanding profiles.
- Define message protocol conventions and error types.
- Define executor integration without adding compiler runtime machinery.
- Add ownership, capacity, cleanup, and cross-target tests.
- Specify supervision and failure handling separately.
