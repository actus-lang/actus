# `std::region` examples

## Open, read, publish, and close

```actus
import std::region;

verb run_region() -> Result[Int, RegionError] {
    erg backing: Buffer = Buffer[4];
    erg opened = region_open[u32](
        backing: dat backing,
        logical_length: 1u64,
        window_start: 0u64,
        window_count: 1u64
    )?;

    erg source: Buffer = Buffer[4];
    region_write(region: ins opened, index: 0u64, source: abs source)?;
    region_publish(region: ins opened)?;

    erg destination: Buffer = Buffer[4];
    region_read(region: abs opened, index: 0u64, destination: ins destination)?;
    region_close(region: ins opened)?;
    return Result[Int, RegionError].Ok(0);
}
```

The buffers are exactly four bytes because `u32` occupies four bytes in this
contract. The sample publishes before reading the committed resident value and
closes the capability explicitly.

## Cancel a mutation

```actus
verb write_then_cancel(ins region: Region[u8], abs source: Buffer) -> Result[Int, RegionError] {
    region_write(region: ins region, index: 0u64, source: abs source)?;
    region_cancel(region: ins region)?;
    return Result[Int, RegionError].Ok(0);
}
```

`region_cancel` restores the most recently published resident bytes.

## Handle an out-of-window access

```actus
verb read_window(abs region: Region[u8], ins destination: Buffer) -> Result[Int, RegionError] {
    return region_read(region: abs region, index: 1u64, destination: ins destination);
}
```

The caller must handle `RegionError.OutOfWindow`; the API does not fetch the
missing element automatically.
