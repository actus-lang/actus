use super::region::RegionError;
use super::serialization::crc32;

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
    StaleGeneration,
    GenerationExhausted,
    ChecksumMismatch,
    Unavailable,
    Cancelled,
    TimedOut,
    Corrupt,
    Truncated,
    Rejected,
    Retryable,
    ReadOnly,
    Terminal,
}

impl From<RegionProviderError> for RegionError {
    fn from(error: RegionProviderError) -> Self {
        match error {
            RegionProviderError::InvalidRequest | RegionProviderError::BufferTooSmall => {
                RegionError::InvalidDescriptor
            }
            RegionProviderError::StaleGeneration => RegionError::StaleGeneration,
            RegionProviderError::GenerationExhausted => RegionError::GenerationExhausted,
            RegionProviderError::Unavailable
            | RegionProviderError::Cancelled
            | RegionProviderError::TimedOut
            | RegionProviderError::Corrupt
            | RegionProviderError::Truncated
            | RegionProviderError::Rejected
            | RegionProviderError::Retryable
            | RegionProviderError::ChecksumMismatch
            | RegionProviderError::ReadOnly
            | RegionProviderError::Terminal => RegionError::BackendFailure,
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
    accepted_generation: u64,
    staged_generation: u64,
    accepted_checksum: u32,
    staged_checksum: u32,
    read_only: bool,
    terminal: bool,
    next_failure: Option<RegionProviderError>,
}

impl MemoryRegionProvider {
    /// Creates a provider with one fixed resident window and no external I/O.
    pub fn new(byte_length: usize) -> Result<Self, RegionProviderError> {
        if byte_length == 0 {
            return Err(RegionProviderError::InvalidRequest);
        }
        let accepted = vec![0; byte_length];
        let checksum = crc32(&accepted);
        Ok(Self {
            accepted,
            staged: vec![0; byte_length],
            accepted_generation: 1,
            staged_generation: 1,
            accepted_checksum: checksum,
            staged_checksum: checksum,
            read_only: false,
            terminal: false,
            next_failure: None,
        })
    }

    /// Injects one explicit provider result for deterministic failure tests.
    pub fn fail_next(&mut self, error: RegionProviderError) {
        self.next_failure = Some(error);
    }

    /// Forces the provider into a readable but non-mutating degraded state.
    pub fn enter_read_only(&mut self) {
        self.read_only = true;
    }

    /// Forces the provider into a terminal state that rejects every operation.
    pub fn enter_terminal(&mut self) {
        self.terminal = true;
    }

    fn validate_buffer(
        &mut self,
        request: RegionProviderRequest,
        length: usize,
    ) -> Result<(), RegionProviderError> {
        request.validate()?;
        self.validate_generation(request.generation)?;
        if request.byte_length != self.accepted.len() || length != request.byte_length {
            return Err(RegionProviderError::BufferTooSmall);
        }
        if self.terminal {
            return Err(RegionProviderError::Terminal);
        }
        if let Some(error) = self.next_failure.take() {
            return Err(error);
        }
        Ok(())
    }

    fn validate_generation(&self, generation: u64) -> Result<(), RegionProviderError> {
        if generation != self.accepted_generation {
            return Err(RegionProviderError::StaleGeneration);
        }
        Ok(())
    }

    fn next_generation(&self) -> Result<u64, RegionProviderError> {
        self.accepted_generation.checked_add(1).ok_or(RegionProviderError::GenerationExhausted)
    }

    fn ensure_writable(&self) -> Result<(), RegionProviderError> {
        if self.terminal {
            return Err(RegionProviderError::Terminal);
        }
        if self.read_only {
            return Err(RegionProviderError::ReadOnly);
        }
        Ok(())
    }

    fn validate_accepted(&self) -> Result<(), RegionProviderError> {
        if self.accepted.len() != self.staged.len() {
            return Err(RegionProviderError::Truncated);
        }
        if crc32(&self.accepted) != self.accepted_checksum {
            return Err(RegionProviderError::ChecksumMismatch);
        }
        Ok(())
    }

    fn validate_staged(&self) -> Result<(), RegionProviderError> {
        if self.staged.len() != self.accepted.len() {
            return Err(RegionProviderError::Truncated);
        }
        if crc32(&self.staged) != self.staged_checksum {
            return Err(RegionProviderError::ChecksumMismatch);
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
        self.validate_accepted()?;
        destination.copy_from_slice(&self.accepted);
        Ok(destination.len() as u64)
    }

    fn store(
        &mut self,
        request: RegionProviderRequest,
        source: &[u8],
    ) -> Result<u64, RegionProviderError> {
        self.validate_buffer(request, source.len())?;
        self.ensure_writable()?;
        let next = self.next_generation()?;
        self.staged.copy_from_slice(source);
        self.staged_checksum = crc32(source);
        self.staged_generation = next;
        Ok(source.len() as u64)
    }

    fn flush(&mut self, request: RegionProviderRequest) -> Result<u64, RegionProviderError> {
        self.validate_operation(request)?;
        self.ensure_writable()?;
        self.validate_accepted()?;
        self.validate_staged()?;
        let next = self.next_generation()?;
        if self.staged_generation != next {
            return Err(RegionProviderError::StaleGeneration);
        }
        self.accepted.copy_from_slice(&self.staged);
        self.accepted_checksum = self.staged_checksum;
        self.accepted_generation = next;
        Ok(0)
    }

    fn cancel(&mut self, request: RegionProviderRequest) -> Result<u64, RegionProviderError> {
        self.validate_operation(request)?;
        self.ensure_writable()?;
        self.validate_accepted()?;
        self.staged.copy_from_slice(&self.accepted);
        self.staged_checksum = self.accepted_checksum;
        self.staged_generation = self.accepted_generation;
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
        let accepted_request = RegionProviderRequest { generation: 2, ..request };
        provider.load(accepted_request, &mut value).expect("load should read flushed bytes");
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

    #[test]
    fn interrupted_flush_preserves_the_previous_generation_and_can_retry() {
        let mut provider = MemoryRegionProvider::new(2).expect("provider should be bounded");
        let first = request(2);
        provider.store(first, &[1, 2]).expect("first store should stage bytes");
        provider.flush(first).expect("first flush should accept bytes");
        let second = RegionProviderRequest { generation: 2, ..first };
        provider.store(second, &[3, 4]).expect("second store should stage bytes");
        provider.fail_next(RegionProviderError::TimedOut);
        assert_eq!(provider.flush(second), Err(RegionProviderError::TimedOut));
        let mut value = [0u8; 2];
        provider.load(second, &mut value).expect("previous generation should remain readable");
        assert_eq!(value, [1, 2]);
        provider.flush(second).expect("retry should accept the staged generation");
        let third = RegionProviderRequest { generation: 3, ..first };
        provider.load(third, &mut value).expect("retried generation should be readable");
        assert_eq!(value, [3, 4]);
    }

    #[test]
    fn checksum_and_truncation_failures_preserve_the_accepted_generation() {
        let mut provider = MemoryRegionProvider::new(2).expect("provider should be bounded");
        let request = request(2);
        provider.store(request, &[7, 8]).expect("store should stage bytes");
        provider.staged[0] ^= 1;
        assert_eq!(provider.flush(request), Err(RegionProviderError::ChecksumMismatch));
        let mut value = [0u8; 2];
        provider.load(request, &mut value).expect("accepted generation should remain readable");
        assert_eq!(value, [0, 0]);

        provider.staged[0] = 7;
        provider.staged.pop();
        assert_eq!(provider.flush(request), Err(RegionProviderError::Truncated));
        assert_eq!(provider.accepted_generation, 1);
    }

    #[test]
    fn read_only_and_terminal_states_have_explicit_transitions() {
        let mut provider = MemoryRegionProvider::new(2).expect("provider should be bounded");
        let request = request(2);
        provider.enter_read_only();
        assert_eq!(provider.store(request, &[1, 2]), Err(RegionProviderError::ReadOnly));
        let mut value = [0u8; 2];
        provider.load(request, &mut value).expect("read-only state remains readable");
        provider.enter_terminal();
        assert_eq!(provider.load(request, &mut value), Err(RegionProviderError::Terminal));
    }

    #[test]
    fn stale_and_exhausted_generations_are_rejected_without_state_change() {
        let mut provider = MemoryRegionProvider::new(2).expect("provider should be bounded");
        let request = request(2);
        let stale = RegionProviderRequest { generation: 2, ..request };
        assert_eq!(provider.store(stale, &[1, 2]), Err(RegionProviderError::StaleGeneration));
        provider.accepted_generation = u64::MAX;
        let exhausted = RegionProviderRequest { generation: u64::MAX, ..request };
        assert_eq!(
            provider.store(exhausted, &[1, 2]),
            Err(RegionProviderError::GenerationExhausted)
        );
        assert_eq!(provider.accepted_generation, u64::MAX);
    }
}
