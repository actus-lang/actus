# The Actus Language Manifesto

**Actus is a low-level systems language built around explicit semantic roles, deterministic ownership, and a small safety core.**

## 1. The Core Idea

Actus expresses how a value participates in an operation through three roles:

- `erg` — an owned, exclusive binding that may read, mutate, borrow, move, or be dropped;
- `abs` — a temporary shared, read-only borrow;
- `dat` — an owned value received through linear ownership transfer.

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

## 5. Compiler Roadmap

The first compiler is bootstrapped in Rust and consists of:

1. a lexer and parser for Actus blocks, verbs, roles, and expressions;
2. an AST and typed semantic analyzer;
3. borrow records and ownership-state checking;
4. deterministic cleanup insertion;
5. a C99 emission backend.

The C backend is an emission target, not the safety authority. Actus safety proofs are completed before code generation.

Later versions may introduce explicitly scoped views or other zero-copy abstractions. Such features must preserve the Alpha ownership guarantees and will not weaken the non-escaping-borrow rule implicitly.

Actus begins with a deliberately small core: predictable ownership, readable systems code, deterministic destruction, and safety rules that can be understood completely.
