# `std::region` runtime and target boundary

The public region API is provider-neutral. A runtime provider may back the
region with in-memory storage, mapped pages, a device, or another bounded
backend, but `Region[T]` source code sees only its checked descriptor contract.

## Provider obligations

A compatible provider must preserve:

- fixed layout validation for `T`;
- logical length versus resident window distinction;
- capability and generation checks;
- checked read, write, publish, cancel, and close behavior;
- deterministic cleanup without leaked handles;
- typed errors for capacity, stale state, bounds, and lifecycle failures.

The provider must not expose raw OS pointers in a region descriptor or make
filesystem access implicit in a read/write operation.

## Hosted and freestanding use

Hosted builds select a configured provider. Freestanding builds need an
explicit target provider; importing the facade alone does not create storage
or prove that a device backend exists.

## Scaling boundary

Logical capacity can exceed resident memory. The compiler/runtime may keep
layout metadata symbolic and use bounded windows, but the public contract does
not promise that every logical element is simultaneously resident. A target
adapter must document its window size, backing storage, publish/cancel cost,
and failure behavior.

## ABI and hot path

Native lowering may pass the small descriptor directly or use an indirect ABI
representation. Actus source must not depend on that choice. Region access is
explicit and must not add hidden filesystem I/O, allocation, or page faults to
a caller's real-time path.
