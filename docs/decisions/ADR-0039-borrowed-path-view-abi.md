# ADR-0039: Borrowed Path-View Return ABI

## Status

Accepted for Phase 16 Gate 3.8 Sub-gate B completion.

## Context

`std::path` exposes `PathComponent` and `PathComponents` as non-owning views.
A component contains the source-path origin and a raw byte or UTF-16 range; it
is not an independently allocated object. Therefore a C bridge must not return
a pointer to a temporary descriptor, a static descriptor, or a newly allocated
buffer.

The existing `Option[abs T]` pointer niche is valid for a borrowed reference to
an existing `T`. It is not sufficient for a derived view whose representation
contains `source`, `offset`, and `length`. Returning such a view as a raw
pointer would lose the range metadata or create an invalid lifetime.

## Decision

Borrowed path-view results use a caller-allocated descriptor slot.

The generated native call allocates a return slot in the caller's active scope.
The runtime C bridge receives the source path and a pointer to that slot. It
writes a descriptor only for `some` results and returns a stable status:

| Status | Meaning |
| ---: | --- |
| `0` | no component exists; the output slot is not observed |
| `1` | a component was written to the output slot |
| `-1` | embedded null or invalid source storage |
| `-2` | invalid platform/code-unit metadata |

The descriptor layout is deterministic and contains:

1. a non-owning source-path pointer;
2. a logical offset in platform units;
3. a logical length in platform units.

`PathComponents` uses the same caller-owned return slot and additionally stores
the current cursor and exclusive payload limit. `next_component(ins iter)`
mutates only that iterator state and writes the next descriptor into its
caller-owned result slot.

The compiler marks the returned descriptor as an `abs` view whose single origin
is the `abs Path` argument. The origin is live only for the returned binding's
scope. The runtime never allocates, retains the source pointer, or converts
POSIX bytes or Windows UTF-16 units to `String`.

## Consequences

- Component and iterator bridges have an explicit out-parameter ABI instead of
  returning a raw descriptor pointer.
- The existing pointer niche remains available for `Option[abs T]` when `T`
  already denotes a reference target; it must not be reused for path-view
  descriptors.
- Codegen must model the hidden result slot and preserve the source-origin
  record through calls, assignments, and scope cleanup.
- Runtime tests must verify `none`, `some`, invalid storage, unchanged source
  bytes, and descriptor lifetime without allocation.

## Implementation order

1. Add a typed native return mode for borrowed view descriptors and a codegen
   test covering the hidden result slot.
2. Add semantic origin propagation and rejected escaping-view tests.
3. Add Rust runtime descriptor writers for parent, file name, stem,
   extension, components, and next component.
4. Replace the current placeholder external declarations with typed Actus
   wrappers and execute POSIX/Windows native integration tests.
