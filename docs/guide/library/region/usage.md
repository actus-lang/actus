# `std::region` usage

## Open a region

`region_open[T]` receives caller-owned backing storage, a logical length, a
resident window start, and a resident window count. `T` must have a fixed
native layout supported by the selected compiler/runtime.

```actus
import std::region;

open verb open_bytes(dat storage: Buffer)
    -> Result[Region[u8], RegionError] {
    return region_open[u8](
        backing: dat storage,
        logical_length: 1024u64,
        window_start: 0u64,
        window_count: 64u64,
    );
}
```

Logical length describes the addressable extent. Window count describes the
resident part available to the current provider. They are separate checked
values.

## Read and write

`region_read` copies one checked element into caller-owned destination storage.
`region_write` validates the source representation and writes one element into
the active region. Neither operation silently expands the logical extent.

## Window lifecycle

A windowed provider must validate the descriptor and generation before access.
Publish commits the active mirror according to the provider contract; cancel
discards the uncommitted mirror; close releases the capability. A stale handle
or generation must return a typed error instead of touching a new region.

## Ownership

Opening consumes backing storage into the region. Access uses explicit loans.
Closing or scope cleanup must release the region capability exactly once.
Callers should not serialize provider handles or assume a physical pointer.

## Scale expectations

Large logical extents require a target/provider designed for windowed access.
The region abstraction does not promise that a one-billion-element extent is
cheap to materialize or that a device can page it without latency; those are
provider evidence requirements.
