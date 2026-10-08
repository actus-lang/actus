# `std::region` public API

```actus
import std::region;
```

## `RegionError`

| Variant | Meaning |
| --- | --- |
| `InvalidDescriptor` | Layout or region metadata does not satisfy the contract. |
| `InvalidHandle` | Capability is unknown or already released. |
| `StaleGeneration` | The supplied capability generation is no longer live. |
| `OutOfWindow` | Index is outside the active resident window. |
| `OffsetOverflow` | Logical index or byte offset cannot be represented. |
| `BufferSize` | Source or destination is not exactly one element wide. |
| `CapabilityExhausted` | Runtime capability table has no free slot. |
| `GenerationExhausted` | Capability generation cannot advance safely. |
| `BackendFailure` | Selected target provider rejected the operation. |

## Read and write

```actus
open verb region_read[T](
    abs region: Region[T],
    erg index: u64,
    ins destination: Buffer
) -> Result[Int, RegionError];

open verb region_write[T](
    ins region: Region[T],
    erg index: u64,
    abs source: Buffer
) -> Result[Int, RegionError];
```

Both buffers must be exactly `size_of[T]()` bytes wide. The runtime derives the
element alignment from `align_of[T]()` and rejects a descriptor whose alignment
is zero, non-power-of-two, or greater than its stride. `region_read` copies one
resident element into the destination. `region_write` replaces one
resident element and makes the region dirty until publish or cancel.

## Lifecycle

```actus
open verb region_publish[T](ins region: Region[T]) -> Result[u64, RegionError];
open verb region_cancel[T](ins region: Region[T]) -> Result[Int, RegionError];
open verb region_close[T](ins region: Region[T]) -> Result[Int, RegionError];
```

`publish` commits the current resident bytes and returns the next generation.
`cancel` restores the latest published bytes. `close` releases the capability;
successful close ends the owner's usable lifecycle.
