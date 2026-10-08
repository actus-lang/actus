# `std::region` ownership and lifecycle

## Ownership roles

| Operation | Role | Meaning |
| --- | --- | --- |
| `region_open` | `dat backing` | Backing storage moves into the new region. |
| `region_read` | `abs region`, `ins destination` | Region is inspected; destination is filled. |
| `region_write` | `ins region`, `abs source` | Region is mutated; source is inspected. |
| `region_publish` | `ins region` | Dirty resident state is committed. |
| `region_cancel` | `ins region` | Dirty resident state is discarded. |
| `region_close` | `ins region` | Capability is released exactly once. |

## Normal lifecycle

1. Allocate or receive a caller-owned backing `Buffer`.
2. Open `Region[T]` with a logical length and resident window.
3. Read or write only indices in the active window.
4. Publish successful mutations or cancel them explicitly.
5. Close the region when the owner leaves its responsibility boundary.

An owner that is moved into a result branch must be handled in that branch.
After successful close, do not read, write, publish, cancel, or close the same
owner again.

## Cleanup

An owned region has compiler-generated cleanup. The runtime release bridge is
called for an active owner at scope cleanup. Explicit `region_close` is the
normal way to release earlier; the compiler must not double-release a region
that was successfully closed.
