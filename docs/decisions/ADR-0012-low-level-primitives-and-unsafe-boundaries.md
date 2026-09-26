# ADR-0012: Low-Level Primitives and Unsafe Boundaries

- Status: Accepted future design
- Date: 2026-09-22
- Scope: Actus low-level memory, hardware access, layout, and unsafe code

## Context

Actus must provide C-level and bare-metal control without weakening the
ownership and borrowing model of ordinary code. Raw memory, hardware
registers, unusual layouts, and target instructions require capabilities that
cannot be verified completely by the safe semantic model.

These capabilities therefore need an explicit unsafe boundary rather than
silently changing the meaning of `erg`, `abs`, or `dat`.

## Decision

Actus has two clearly separated low-level modes:

```text
safe Actus
    erg / abs / dat / ins
    ownership and borrowing
    deterministic cleanup

unsafe Actus
    raw pointers and casts
    MMIO and volatile access
    unaligned memory
    inline assembly
    target-specific operations
```

Safe code retains the normal ownership guarantees. Unsafe code may bypass
some guarantees, but the bypass must be explicit and locally visible.

## Raw Pointers

Actus provides unmanaged raw pointer types:

```act
*const T
*mut T
```

A raw pointer binding is not an owner and therefore has no semantic role and
no destructor responsibility:

```act
unsafe {
    reg: *mut u32 = addr(0x40011000);
    write(reg, 1);
}
```

Raw pointer bindings must not be declared as `erg`. They are unmanaged
bindings, are not automatically dropped, and do not transfer ownership merely
because they are copied or passed to an intrinsic.

The initial raw pointer intrinsics are:

```text
addr(value)
cast(pointer)
read(pointer)
write(pointer, value)
offset(pointer, count)
```

Pointer dereference, pointer arithmetic, casts that may change validity, and
conversion of a borrow to a raw pointer require an unsafe boundary. Raw
pointers do not extend the lifetime of an `abs` borrow and do not make a
borrowed value safe to return or store.

## Hardware and MMIO

The `core` layer provides dedicated intrinsics for hardware memory access:

```text
volatile_read(pointer)
volatile_write(pointer, value)
```

Volatile operations are distinct from ordinary memory operations and must not
be implemented as ordinary user-defined functions whose hardware semantics
the compiler cannot observe.

The core low-level synchronization set may also include:

```text
memory_fence
compiler_fence
atomic_load
atomic_store
atomic_exchange
compare_exchange
```

The contracts are distinct:

- ordinary reads and writes access normal memory;
- volatile operations access externally observable memory such as MMIO;
- atomics provide synchronization for shared memory;
- fences constrain compiler and hardware ordering.

The selected target defines the supported widths, ordering modes, and
availability of each operation.

## Memory Layout and Alignment

Actus uses explicit attributes for externally constrained layouts:

```act
meta repr(C)
open struct RegisterBlock {
    control: u32,
    status: u32,
}

meta repr(transparent)
struct Handle {
    raw: *mut u8,
}

meta packed
struct WireHeader {
    kind: u8,
    length: u32,
}

meta align(64)
struct CacheLine {
    ...
}
```

The attributes mean:

- `meta repr(C)` defines C-compatible field order and layout rules;
- `meta repr(transparent)` preserves the ABI of the underlying representation;
- `meta packed` removes padding and makes unaligned access a caller responsibility;
- `meta align(N)` raises the required alignment to at least `N`.

The compiler provides compile-time layout queries and assertions:

```text
size_of<T>()
align_of<T>()
offset_of<T, field>()
assert_size<T>(expected)
assert_align<T>(expected)
assert_offset<T, field>(expected)
```

Layout mismatches must fail compilation before code generation. Packed and
unaligned accesses require an explicit unsafe boundary where the normal
alignment guarantee does not apply.

## Inline Assembly

Inline assembly is available only in an unsafe boundary:

```act
unsafe {
    asm("nop");
}
```

The assembly interface must model inputs, outputs, clobbers, memory effects,
volatility, and target constraints. Cranelift remains responsible for normal
native instruction generation. Inline assembly is reserved for target
instructions or effects that the backend cannot represent portably.

If the selected target or backend cannot support a requested assembly block,
the compiler must report a deterministic diagnostic rather than silently
emitting a different instruction sequence.

Assembly blocks have no automatic ownership guarantees. Pointer and memory
effects must be declared through the assembly contract and remain inside the
unsafe boundary.

## Boundary Rules

- Safe Actus uses `erg`, `abs`, and `dat` with normal ownership semantics.
- Raw pointer bindings have no ownership role and cannot trigger cleanup.
- Unsafe operations must be explicit and locally scoped.
- Unsafe code does not silently make `abs` borrows escape.
- The compiler must preserve diagnostics and source spans across unsafe blocks.
- The stable C ABI remains the supported external binary boundary defined by
  the separate FFI decisions.

## Consequences

Actus can express raw memory access, MMIO, exact layouts, synchronization, and
target instructions without turning ordinary programs into unchecked C-like
code. The unmanaged pointer model avoids falsely treating addresses as owned
resources, while explicit unsafe boundaries make the remaining obligations
visible to programmers and reviewers.
