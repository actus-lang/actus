mod access;
mod bridge;
mod descriptor;
mod profile;
mod provider;

#[cfg(test)]
mod tests;

pub use access::{InMemoryRegion, MutableRegionView, RegionView};
pub use bridge::{
    actus_region_drop, actus_region_pin, actus_region_pinned, actus_region_read_range,
    actus_region_unpin, actus_region_volatile_read_range, actus_region_volatile_write_range,
    actus_region_write_range,
};
pub use descriptor::{
    RegionDescriptor, RegionError, RegionGeneration, RegionHandle, UNINITIALIZED_REGION_GENERATION,
};
pub use profile::RegionAddressProfile;
pub use provider::{
    MemoryRegionProvider, RegionProvider, RegionProviderError, RegionProviderRequest,
};
