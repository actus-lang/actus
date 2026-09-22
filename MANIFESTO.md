# The Actus Language Manifesto

**Actus is a low-level systems language built around explicit semantic roles, deterministic ownership, and a small safety core.**

Actus is a low-level language that makes ownership roles, borrowing boundaries,
task lifetimes, and cleanup behavior explicit without requiring garbage
collection or user-written lifetime annotations.

## 1. The Core Idea

Actus expresses how a value participates in an operation through three roles:

- `erg` — an owned, exclusive binding that may read, mutate, borrow, move, or be dropped;
- `abs` — a temporary shared, read-only borrow;
- `dat` — an ownership transfer role: the caller binding becomes `Moved`, and
  the callee or task becomes the new owner.

These roles are enforced by the compiler. They are not documentation conventions.

## 2. Alpha Ownership Model

Actus Alpha uses lexical, non-escaping borrows only.

- An `abs` borrow cannot be returned from a function.
- An `abs` borrow cannot be stored in a struct or transferred into a longer-lived object.
- An `abs` borrow may be passed to a nested function call for the duration of that call.
- No lifetime annotations are exposed to the programmer.

An owner with one or more active shared borrows is derived as `Frozen`. While frozen, it cannot be mutated, moved, or dropped. When the last borrow ends at its lexical scope boundary, the owner becomes `Active` again.

The compiler tracks active borrows through borrow records containing the borrow identity, owner, scope, and source location. `Frozen` is derived from those records and is never an independent user-visible state.

## 3. Ownership and Cleanup

Function results use value semantics by default:

```act
verb create_buffer() -> Buffer {
    erg buffer = allocate(512);
    return buffer;
}
```

Returning a value transfers ownership to the caller. Borrowed returns are not part of the Alpha language.

`drop(value)` is a compiler intrinsic. It immediately destroys an owned resource and marks its binding as `Dropped`. Automatic cleanup never drops a resource twice. `drop(abs_borrow)` is invalid; borrows end automatically at their lexical scope boundary.

Every explicit block creates a scope. On scope exit, remaining owned resources are destroyed in reverse declaration order. `return`, `break`, and `continue` unwind the required scopes before transferring control.

## 4. Syntax and Operations

Actus uses explicit C-style braces and semicolons:

```act
verb process() {
    erg buffer = allocate(1024);
    {
        abs view = ref buffer;
        inspect(view);
    }
    buffer.append(42);
}
```

Verbs coordinate actants instead of attaching behavior exclusively to one object. Parameters are declared with semantic roles, and arguments may be positional or explicitly named. Ambiguous positional calls must use names.

## 5. Structured Concurrency

Actus uses structured concurrency by default. Concurrent execution is an
ownership operation, not an implicit background side effect:

```act
erg task = act(process, dat payload);
join(dat task);
```

`act` transfers ownership of `payload` into a new task and returns an owned
`erg Task` handle. The caller binding becomes `Moved`; the task becomes the
new owner and deterministically cleans up the payload when execution ends.

Task handles are owned resources and must be explicitly consumed:

- `join(dat task)` waits for completion and consumes the task handle;
- `detach(dat task)` explicitly transfers responsibility to the scheduler.

Fire-and-forget execution is never the implicit behavior of `act`. The
freestanding `core` layer does not provide a general task runtime. Hosted
profiles lower task operations through the standard library, while a target
may provide an explicit scheduler contract.

## 6. Why Actus

Actus is not intended to replace every systems language. Its focus is a
smaller and more explicit ownership model for low-level programs.

### Compared with Rust

Actus uses lexical, non-escaping borrows and exposes no lifetime annotations
in source code. This makes the initial ownership model easier to read and
diagnose, but it is intentionally less expressive than Rust's lifetime
system. Alpha Actus does not support borrowed returns or arbitrary lifetime
relationships.

### Compared with C

C provides broad ABI compatibility and unrestricted low-level control, but
ownership and cleanup are primarily programmer conventions. Actus adds
compile-time checks for moves, borrows, use-after-move, double-drop, and
deterministic scope cleanup while preserving a stable C FFI boundary.

### Compared with Zig

Zig provides explicit low-level control and allocator-aware programming. Actus
adds a role-based ownership and borrowing model in which `erg`, `abs`, and
`dat` make resource relationships visible in declarations and calls. This
reduces some classes of lifetime errors, at the cost of restricting borrowed
values from escaping in Alpha.

## 7. Design Boundaries and Trade-offs

Actus Alpha deliberately chooses a restricted safety model instead of trying
to reproduce every capability of mature systems languages.

- Borrowed values cannot be returned from functions.
- Borrows cannot be stored in longer-lived structures.
- Complex lifetime relationships are not exposed to programmers.
- Structured concurrency requires explicit task lifecycle handling.
- Detached tasks require an explicit `detach` operation.
- The internal Actus unit ABI is unstable during Alpha.
- C is a stable FFI/ABI boundary, not a required compiler intermediate
  representation or safety authority.

These restrictions are intentional. They keep the compiler model predictable
while leaving room for future scoped views, richer task results, and other
zero-copy abstractions that preserve the core ownership guarantees.

## 8. Compiler Roadmap

The first compiler is bootstrapped in Rust and consists of:

1. a lexer and parser for Actus blocks, verbs, roles, and expressions;
2. an AST and typed semantic analyzer;
3. borrow records and ownership-state checking;
4. deterministic cleanup insertion;
5. a Rust-native Cranelift backend for native object and executable emission.

The C ABI is a stable interoperability boundary for external libraries,
operating-system interfaces, and platform runtimes. It is not required as an
intermediate representation, and it is not the safety authority. Actus
semantic safety checks are completed before code generation.

Later versions may introduce explicitly scoped views or other zero-copy abstractions. Such features must preserve the Alpha ownership guarantees and will not weaken the non-escaping-borrow rule implicitly.

Actus begins with a deliberately small core: predictable ownership, readable
systems code, deterministic destruction, explicit task lifetimes, and safety
rules that can be understood completely.
