# ADR-0074: Source-Level Runtime Region Type and Native ABI

## Status

Accepted for implementation planning.

## Context

ADR-0073 defines a runtime-backed logical region as a separate representation
from `Array[T, N]`. Gate 37.6 requires a native ABI and executable coverage,
but the language currently has no source-level type that can name this
resource. Treating a region as an array would silently change contiguous
storage, cleanup, indexing, and call-boundary semantics. Treating it as an
untyped buffer would discard the element layout identity required for checked
offset calculation.

The compiler therefore needs a source contract before native lowering can be
implemented. The contract must keep the public Actus model target-neutral,
preserve ownership roles, and avoid exposing a pointer or an implicit page
fault.

## Decision

### Public type

Introduce `Region[T]` as a first-class runtime-backed resource type with one
type argument. `T` identifies the element layout and semantic type. The type
does not contain the resident elements inline and is not an alternate spelling
for `Array[T, N]`.

`Region[T]` has no implicit indexing, iteration, conversion to `Array[T, N]`,
filesystem access, page fault, or allocation. All access uses explicit region
operations that return typed success or failure results.

### Native representation

The first native representation is a compact value equivalent to the reviewed
`RegionDescriptor`:

- fixed-width `u64` capability handle;
- fixed-width element stride and logical length;
- fixed-width resident window start and count;
- fixed-width publication generation;
- fixed-width dirty and address-profile fields;
- no operating-system pointer or process address.

The compiler lowers the descriptor as one bounded ABI value. It must not
specialize a function or symbol per logical element and must not materialize
resident storage from logical capacity. `Region[T]` and `Array[T, N]` have
different semantic identities, layout identities, and call-boundary contracts.

### Ownership

- `erg Region[T]` is the mutable owner and cleanup authority.
- `abs Region[T]` is a read-only region operation view and cannot mutate,
  publish, evict, or reuse the region.
- `ins Region[T]` is the exclusive call-scoped mutable loan used by explicit
  mutation or publication operations.
- `dat Region[T]` transfers terminal ownership through an operation that
  documents whether the capability is closed or transferred.

An `abs` or `ins` view cannot escape its call scope. The compiler must reject
aliasing, use after move, publication while a view is live, and an owned region
stored inside a borrowed view. The runtime still validates handle and
generation because type checking alone cannot validate stale external state.

### Initial operation surface

The first language integration exposes only explicit operations:

1. create or open a region from a checked descriptor or backend capability;
2. borrow a bounded resident window;
3. read or write one element through a checked logical index;
4. publish or cancel the current dirty window;
5. close or transfer the owned resource.

The operation surface must use typed `Result` failures for invalid handles,
stale generations, unavailable windows, bounds errors, arithmetic overflow,
buffer-size mismatch, and publication failure. No valid input may reach a
compiler-generated trap.

### Lowering boundary

The compiler implementation proceeds in this order:

1. register `Region` as a distinct generic semantic type with arity one;
2. define its structural identity and role compatibility rules;
3. add target layout and ABI mapping for the bounded descriptor value;
4. add private runtime bridges for explicit operations;
5. expose only facade-approved public verbs;
6. add native executable tests for read, write, bounds failure, publication,
   cleanup, symbol boundedness, and inline-array ABI separation.

The runtime bridge remains private to the standard library/runtime facade.
Application modules cannot import raw bridge symbols or bypass the canonical
facade.

## Rejected alternatives

- **`Array[T, N]` alias:** changes inline storage and ABI semantics.
- **`Buffer` alias:** loses typed element layout and region generation rules.
- **Raw pointer field:** violates position independence and target neutrality.
- **Implicit indexing:** hides window availability and can introduce hidden
  I/O or unbounded latency.
- **Compiler-only magic without a source type:** makes ownership and cleanup
  invisible at call boundaries.

## Consequences

The compiler gains a precise path to support logical capacities larger than
resident memory while preserving the existing array contract. The first
implementation is intentionally explicit and serialized. Convenience
iteration, concurrent views, mapped storage, and target-specific acceleration
remain later decisions.

Until the semantic type, ABI mapping, and private bridges are implemented,
Gate 37.6 must remain open. The hosted Rust runtime descriptor is valid
runtime evidence, but it is not evidence that Actus source can already declare
or execute `Region[T]`.

## Verification contract

- Semantic tests distinguish `Region[T]` from `Array[T, N]`.
- Role tests cover `erg`, `abs`, `ins`, and `dat` lifecycle rules.
- Layout tests assert one bounded descriptor ABI independent of logical length.
- Native tests execute explicit read, write, bounds rejection, publication, and
  cleanup paths.
- Object tests verify bounded symbols, relocations, sections, and output size.
- Invalid descriptor and stale-generation paths return typed failures without
  compiler-generated traps.
