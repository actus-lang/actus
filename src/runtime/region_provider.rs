use super::region::RegionError;

/// Fixed-width request shared by every Region storage provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionProviderRequest {
    pub handle: u64,
    pub generation: u64,
    pub window_start: u64,
    pub window_count: u64,
    pub byte_length: usize,
}

impl RegionProviderRequest {
    /// Validates the provider-independent identity and bounded byte extent.
    pub fn validate(&self) -> Result<(), RegionProviderError> {
        if self.handle == 0 || self.generation == 0 || self.window_count == 0 {
            return Err(RegionProviderError::InvalidRequest);
        }
        if self.window_start.checked_add(self.window_count).is_none() {
            return Err(RegionProviderError::InvalidRequest);
        }
        if self.byte_length == 0 {
            return Err(RegionProviderError::InvalidRequest);
        }
        Ok(())
    }
}

/// Explicit provider failures that do not expose platform status codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionProviderError {
    InvalidRequest,
    BufferTooSmall,
    Unavailable,
    Cancelled,
    TimedOut,
    Corrupt,
    Truncated,
    Rejected,
    Retryable,
}

impl From<RegionProviderError> for RegionError {
    fn from(error: RegionProviderError) -> Self {
        match error {
            RegionProviderError::InvalidRequest | RegionProviderError::BufferTooSmall => {
                RegionError::InvalidDescriptor
            }
            RegionProviderError::Unavailable
            | RegionProviderError::Cancelled
            | RegionProviderError::TimedOut
            | RegionProviderError::Corrupt
            | RegionProviderError::Truncated
            | RegionProviderError::Rejected
            | RegionProviderError::Retryable => RegionError::BackendFailure,
        }
    }
}

/// Target-neutral bounded provider operations.
pub trait RegionProvider {
    /// Loads the last accepted bytes into caller-owned resident storage.
    fn load(
        &mut self,
        request: RegionProviderRequest,
        destination: &mut [u8],
    ) -> Result<u64, RegionProviderError>;

    /// Stores caller-owned bytes in provider staging storage.
    fn store(
        &mut self,
        request: RegionProviderRequest,
        source: &[u8],
    ) -> Result<u64, RegionProviderError>;

    /// Accepts staged bytes as the provider's durable generation.
    fn flush(&mut self, request: RegionProviderRequest) -> Result<u64, RegionProviderError>;

    /// Discards staged bytes and restores the last accepted generation.
    fn cancel(&mut self, request: RegionProviderRequest) -> Result<u64, RegionProviderError>;

    /// Loads the last accepted generation after a provider interruption.
    fn recover(
        &mut self,
        request: RegionProviderRequest,
        destination: &mut [u8],
    ) -> Result<u64, RegionProviderError>;
}

/// Deterministic fixed-capacity provider used by hosted tests and examples.
pub struct MemoryRegionProvider {
    accepted: Vec<u8>,
    staged: Vec<u8>,
    next_failure: Option<RegionProviderError>,
}

impl MemoryRegionProvider {
    /// Creates a provider with one fixed resident window and no external I/O.
    pub fn new(byte_length: usize) -> Result<Self, RegionProviderError> {
        if byte_length == 0 {
            return Err(RegionProviderError::InvalidRequest);
        }
        Ok(Self {
            accepted: vec![0; byte_length],
            staged: vec![0; byte_length],
            next_failure: None,
        })
    }

    /// Injects one explicit provider result for deterministic failure tests.
    pub fn fail_next(&mut self, error: RegionProviderError) {
        self.next_failure = Some(error);
    }

    fn validate_buffer(
        &mut self,
        request: RegionProviderRequest,
        length: usize,
    ) -> Result<(), RegionProviderError> {
        request.validate()?;
        if request.byte_length != self.accepted.len() || length != request.byte_length {
            return Err(RegionProviderError::BufferTooSmall);
        }
        if let Some(error) = self.next_failure.take() {
            return Err(error);
        }
        Ok(())
    }

    fn validate_operation(
        &mut self,
        request: RegionProviderRequest,
    ) -> Result<(), RegionProviderError> {
        self.validate_buffer(request, request.byte_length)
    }
}

impl RegionProvider for MemoryRegionProvider {
    fn load(
        &mut self,
        request: RegionProviderRequest,
        destination: &mut [u8],
    ) -> Result<u64, RegionProviderError> {
        self.validate_buffer(request, destination.len())?;
        destination.copy_from_slice(&self.accepted);
        Ok(destination.len() as u64)
    }

    fn store(
        &mut self,
        request: RegionProviderRequest,
        source: &[u8],
    ) -> Result<u64, RegionProviderError> {
        self.validate_buffer(request, source.len())?;
        self.staged.copy_from_slice(source);
        Ok(source.len() as u64)
    }

    fn flush(&mut self, request: RegionProviderRequest) -> Result<u64, RegionProviderError> {
        self.validate_operation(request)?;
        self.accepted.copy_from_slice(&self.staged);
        Ok(0)
    }

    fn cancel(&mut self, request: RegionProviderRequest) -> Result<u64, RegionProviderError> {
        self.validate_operation(request)?;
        self.staged.copy_from_slice(&self.accepted);
        Ok(0)
    }

    fn recover(
        &mut self,
        request: RegionProviderRequest,
        destination: &mut [u8],
    ) -> Result<u64, RegionProviderError> {
        self.load(request, destination)
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryRegionProvider, RegionProvider, RegionProviderError, RegionProviderRequest};

    fn request(byte_length: usize) -> RegionProviderRequest {
        RegionProviderRequest {
            handle: 7,
            generation: 1,
            window_start: 0,
            window_count: 1,
            byte_length,
        }
    }

    #[test]
    fn memory_provider_separates_staging_from_accepted_bytes() {
        let mut provider = MemoryRegionProvider::new(4).expect("provider should be bounded");
        let request = request(4);
        provider.store(request, &[1, 2, 3, 4]).expect("store should stage bytes");
        let mut value = [0u8; 4];
        provider.load(request, &mut value).expect("load should read accepted bytes");
        assert_eq!(value, [0, 0, 0, 0]);
        provider.flush(request).expect("flush should accept staging");
        provider.load(request, &mut value).expect("load should read flushed bytes");
        assert_eq!(value, [1, 2, 3, 4]);
    }

    #[test]
    fn memory_provider_cancel_and_recover_keep_the_last_accepted_generation() {
        let mut provider = MemoryRegionProvider::new(2).expect("provider should be bounded");
        let request = request(2);
        provider.store(request, &[5, 6]).expect("store should stage bytes");
        provider.cancel(request).expect("cancel should discard staging");
        let mut value = [0u8; 2];
        provider.recover(request, &mut value).expect("recover should load accepted bytes");
        assert_eq!(value, [0, 0]);
    }

    #[test]
    fn memory_provider_reports_explicit_failures_without_mutating_state() {
        let mut provider = MemoryRegionProvider::new(2).expect("provider should be bounded");
        let request = request(2);
        provider.store(request, &[9, 8]).expect("store should stage bytes");
        provider.fail_next(RegionProviderError::TimedOut);
        assert_eq!(provider.flush(request), Err(RegionProviderError::TimedOut));
        let mut value = [0u8; 2];
        provider.load(request, &mut value).expect("accepted bytes should remain readable");
        assert_eq!(value, [0, 0]);
        let mapped: super::RegionError = RegionProviderError::Corrupt.into();
        assert_eq!(mapped, super::RegionError::BackendFailure);
    }

    #[test]
    fn memory_provider_rejects_wrong_capacity_before_copying() {
        let mut provider = MemoryRegionProvider::new(4).expect("provider should be bounded");
        let request = request(4);
        assert_eq!(provider.store(request, &[1, 2]), Err(RegionProviderError::BufferTooSmall));
    }
}
