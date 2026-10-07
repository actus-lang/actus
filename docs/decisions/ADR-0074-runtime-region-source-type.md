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

`T` must be a fully resolved sized type with a checked, fixed native layout.
The semantic analyzer rejects incomplete, unsized, dynamically sized, or
opaque element types before generic registration or native lowering. A region
may contain a fixed-size struct or array only when every nested field and
element is itself sized. Dynamic `String`, `Buffer`, `Map`, and another
runtime-backed region are not valid direct element types in the first profile.

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

An owned `erg Region[T]` registers a cleanup action at initialization. The
normal LIFO teardown path lowers that action to the private runtime close/drop
bridge, which releases the capability slot exactly once. Moves remove the
source cleanup action; failed initialization does not register one; and
explicit close transfers or consumes the cleanup responsibility according to
the operation contract. A region must never leave a live capability slot
behind merely because its lexical scope ended.

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

### Descriptor call ABI

The descriptor is a small fixed aggregate, but its six `u64` words plus
metadata must not be assumed to occupy the same registers on every target.
The compiler uses the target ABI's aggregate classification and selects
indirect descriptor passing when the target cannot pass the complete value
directly. Role-qualified calls receive the appropriate generated address:
`abs` is read-only, `ins` is exclusive mutable, and ownership cleanup remains
attached to the owner rather than to the temporary call representation.

The indirect address exists only at the native call boundary. It is not stored
in the Actus `Region[T]` value, serialized metadata, capability handle, or
public source model. Cranelift lowering must therefore use the target layout
registry and ABI classification instead of manually splitting the descriptor
into assumed registers. Tests must cover both direct and indirect aggregate
classification where the configured target profiles differ.

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

The semantic type, descriptor layout mapping, capability table, and private
cleanup bridge are now implemented as the first compiler boundary. Gate 37.6
must remain open until explicit create/read/write/publish operations and
target-specific aggregate classification are implemented and tested. The
current native evidence proves declaration and owned cleanup lowering only;
it does not prove that Actus source can already execute region operations.

## Verification contract

- Semantic tests distinguish `Region[T]` from `Array[T, N]`.
- Semantic tests reject unsized or dynamically sized region element types.
- Role tests cover `erg`, `abs`, `ins`, and `dat` lifecycle rules.
- Cleanup tests prove that lexical teardown emits one capability-release bridge
  call and that move/explicit-close paths do not double-release.
- Layout tests assert one bounded descriptor ABI independent of logical length.
- ABI tests cover target aggregate classification and indirect descriptor
  passing without exposing a pointer in the public representation.
- Native tests execute explicit read, write, bounds rejection, publication, and
  cleanup paths.
- Object tests verify bounded symbols, relocations, sections, and output size.
- Invalid descriptor and stale-generation paths return typed failures without
  compiler-generated traps.
