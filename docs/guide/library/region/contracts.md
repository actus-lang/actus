# `std::region` bounds, generations, and failure contracts

## Bounds

An index must satisfy both conditions:

1. it is representable by the target integer contract;
2. it lies inside the logical extent and active resident window.

The second condition is why a logically valid index can still return
`OutOfWindow`. The API does not implicitly page, fetch, or extend storage.

## Exact element buffers

`region_read` and `region_write` operate on exactly one `T`. A destination or
source with a different length returns `BufferSize`. This keeps element stride,
copy bounds, and native layout aligned.

## Generation behavior

Publishing advances the live generation. Cancel restores the latest published
state. Any stale descriptor or provider operation from an older generation
returns `StaleGeneration`. Generation exhaustion is a typed terminal failure;
the counter must not wrap into a valid old generation.

## Failure handling

| Failure | Correct response |
| --- | --- |
| `InvalidDescriptor` | Fix the element/layout or opening metadata. |
| `InvalidHandle` | Stop using the released or unknown capability. |
| `OutOfWindow` | Move/replace the resident window through the target provider. |
| `BufferSize` | Supply exactly one element's storage. |
| `CapabilityExhausted` | Release unused capabilities or report resource exhaustion. |
| `BackendFailure` | Handle the target provider failure; do not reinterpret it as success. |

No failure path creates an implicit filesystem operation or silently changes
the region's logical length.
