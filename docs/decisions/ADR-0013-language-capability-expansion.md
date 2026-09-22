# ADR-0013: Language Capability Expansion for Self-Hosting and Physical AI

- Status: Accepted future design
- Date: 2026-09-22
- Scope: Actus data modeling, control flow, collections, mathematics, and core types

## Context

Actus must become capable of compiling its own compiler while remaining useful
for robotics and Physical AI. The language therefore needs algebraic data
modeling, typed error handling, deterministic collections, compile-time
numeric information, and fixed-size mathematical values without weakening the
`erg`/`abs`/`dat` ownership model or introducing garbage collection.

## Core Principles

All new constructs preserve the existing roles:

- `erg` owns a value or resource and is responsible for mutation and cleanup;
- `abs` provides temporary shared, read-only access within a lexical scope;
- `dat` transfers ownership, moving the caller binding and creating a new owner
  in the callee or receiving context.

Heap-backed values use deterministic ownership and LIFO cleanup. Fixed-size
values may live on the stack. No feature introduces an implicit garbage
collector or hidden ownership transfer.

## Core Types

The initial primitive type family is:

```text
Bool                 true, false, logical operations, and conditions
Int                  default signed integer type
I8, I16, I32, I64    explicit signed integer widths
U8, U16, U32, U64    explicit unsigned integer widths
Usize                pointer-width unsigned type for addresses and indices
F32, F64             floating-point types for numerical and vector mathematics
```

The compiler must reject implicit conversions that could lose precision or
change signedness unexpectedly. Numeric literals are checked against the
expected type and target width before code generation.

## Structs and Field Ownership

`struct` defines a typed aggregate:

```act
struct Point {
    x: F32,
    y: F32,
}

struct Packet {
    erg payload: Buffer,
    sequence: U32,
}
```

Value fields use value semantics. Resource fields may be explicitly marked
`erg` and participate in deterministic reverse-order cleanup. `dat` describes
transfer operations and is not a persistent field role. Alpha does not allow
`abs` borrows to be stored in longer-lived structs.

Moving a struct transfers its owned fields. Dropping a struct drops remaining
owned fields in reverse declaration order. Struct layout attributes and
external representation remain governed by ADR-0012.

## Enums and Algebraic Data Types

`enum` provides tagged unions with value-carrying variants:

```act
enum Expr {
    Integer(Int),
    Name(Symbol),
    Binary {
        erg left: Expr,
        erg right: Expr,
    },
}
```

Variant payloads have normal ownership semantics. Constructing a variant may
move owned values into it. Inspection creates lexical `abs` access; consuming
patterns use explicit `dat` transfer.

Enums are required for compiler ASTs, parser states, diagnostics, state
machines, and typed robotics results.

## Option and Result

Actus provides standard algebraic types without null pointers or exception
runtime behavior:

```act
enum Option[T] {
    Some(T),
    None,
}

enum Result[T, E] {
    Ok(T),
    Err(E),
}
```

`Option[T]` represents presence or absence. `Result[T, E]` represents success
or a typed error. Both are ordinary owned values with deterministic cleanup.

## `case` Pattern Matching and Switching

`case` performs exhaustive pattern matching for enums and tagged values:

```act
case result {
    Ok(value) => use(value),
    Err(error) => report(error),
}
```

Inspection and consumption are explicit:

```act
case abs expression {
    Integer(value) => inspect(value),
    Name(symbol) => inspect(symbol),
}

case dat expression {
    Binary { left, right } => consume(left, right),
}
```

`case` also replaces the classic numeric or literal `switch`:

```act
case status {
    0 => idle(),
    1 => running(),
    _ => report_unknown(status),
}
```

Matching has no fallthrough and no implicit `break`. The `_` wildcard handles
the remaining cases. Enum matches must be exhaustive; primitive matches must
either cover all required values or provide `_`.

Pattern bindings do not copy owned payloads implicitly. Unconsumed owned
values remain subject to normal scope cleanup.

## Iteration

`for ... in` supports ranges and collections:

```act
for index in 0..10 {
    process(index);
}

for abs item in items {
    inspect(item);
}
```

Collection iteration yields lexical `abs` access by default. The collection
owner is `Frozen` for the iteration and mutation or reallocation during the
iteration is rejected. Alpha does not define implicit mutable iteration.

## Error Propagation

The `?` operator propagates `Err` values from a `Result`-returning verb:

```act
verb compile(abs source: String) -> Result[Binary, CompileError] {
    erg tokens = lex(source)?;
    erg ast = parse(tokens)?;
    return lower(ast);
}
```

An `Ok` value is extracted and execution continues. An `Err` value returns
immediately after all active scopes are unwound. LIFO cleanup, moved-resource
handling, and borrow termination are identical to explicit `return`.

## Collections and Zero-Copy Views

The standard collection family includes:

- `Buffer` for owned byte-oriented storage;
- `Array[T]` for dynamic owned contiguous values;
- `Map[K, V]` for associative lookup and compiler symbol tables;
- `Slice[T]` for non-owning zero-copy views.

The canonical buffer initialization syntax is:

```act
erg buffer = Buffer[1024];
```

`Slice[T]` is always an `abs` lexical view. It does not own memory, cannot be
returned, cannot be stored in a longer-lived structure, and cannot be moved
with `dat`:

```act
abs view: Slice[U8] = buffer.slice(0, 128);
```

The owner becomes `Frozen` while the view is active and returns to `Active`
when the view's lexical scope ends. Collection implementations must preserve
deterministic compiler behavior; symbol-table iteration and diagnostics must
not depend on unstable hash iteration order.

## Fixed Mathematical Types

Actus provides fixed-size stack-oriented mathematical types:

```act
erg position: Vector[F32, 3];
erg rotation: Matrix[F32, 3, 3];
erg transform: Matrix[F32, 4, 4];
```

Dimensions are compile-time values. These types support robotics kinematics,
coordinate transforms, sensor processing, and SIMD-friendly layouts without
requiring heap allocation. `abs` parameters provide read-only views, `erg`
values may be mutated, and `dat` transfers ownership where required.

SIMD instruction selection is a backend optimization. The language semantics
of `Vector` and `Matrix` remain valid on targets without SIMD support.

## Compile-Time Constants

`const` defines immutable compile-time values:

```act
const MAX_JOINTS: Int = 12;
const DOF: Int = 6;
const MMIO_BASE: Usize = 0x40011000;
```

Constants may be used for dimensions, range bounds, addresses supplied by
external platform libraries, layout assertions, and static invariants. The
constant evaluator is allocation-free, side-effect-free, typed, and must
reject invalid expressions before code generation.

## Concurrency Boundary

Actus does not introduce `async`/`await`. It avoids function coloring, hidden
state machines, and an implicit future runtime. Concurrency remains within the
structured model defined by ADR-0005:

```act
erg task = act(process, dat payload);
join(dat task);
```

Task handles are owned resources. `join` and explicit `detach` are the only
task lifecycle operations. No concurrent construct may silently create an
unmanaged background task.

## Self-Hosting and Physical AI Priority

The self-hosting core consists of `struct`, `enum`, `Option`, `Result`,
exhaustive `case`, `?`, collections, generics, and compile-time evaluation.
These features are required for a practical Actus lexer, parser, AST,
semantic analyzer, diagnostics system, and package tooling.

Fixed vectors, matrices, zero-copy slices, and the low-level primitives from
ADR-0012 extend the same model to robotics, numerical computation, and
hardware-adjacent applications.

## Consequences

Actus gains the data modeling and control-flow expressiveness required for
self-hosting without adding a garbage collector or weakening deterministic
ownership. Robotics programs receive fixed-size mathematical values and
zero-copy views, while low-level memory access remains explicitly separated
behind the unsafe boundary defined by ADR-0012.

