use std::fmt::{Display, Formatter};

use super::{TimeProviderContract, WrapBehavior};

/// Errors produced when a bounded hardware counter cannot be extended safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CounterSampleError {
    BackwardDiscontinuity,
    ExtendedValueOverflow,
}

impl Display for CounterSampleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BackwardDiscontinuity => {
                formatter.write_str("counter moved backwards without a wrap")
            }
            Self::ExtendedValueOverflow => formatter.write_str("extended counter exceeded u64"),
        }
    }
}

impl std::error::Error for CounterSampleError {}

/// Deterministically extends a provider counter into a monotonic `u64` domain.
#[derive(Clone, Debug)]
pub struct CounterExtender {
    width: u8,
    wrap_behavior: WrapBehavior,
    previous_raw: Option<u64>,
    extended: u128,
}

impl CounterExtender {
    pub fn new(provider: &TimeProviderContract) -> Self {
        Self {
            width: provider.counter_width(),
            wrap_behavior: provider.wrap_behavior(),
            previous_raw: None,
            extended: 0,
        }
    }

    pub fn observe(&mut self, raw: u64) -> Result<u64, CounterSampleError> {
        let modulus = 1_u128 << self.width;
        let raw = u128::from(raw) & (modulus - 1);
        let Some(previous_raw) = self.previous_raw else {
            self.previous_raw = Some(raw as u64);
            self.extended = raw;
            return u64::try_from(self.extended)
                .map_err(|_| CounterSampleError::ExtendedValueOverflow);
        };
        let previous = u128::from(previous_raw);
        let next = match self.wrap_behavior {
            WrapBehavior::Reject if raw < previous => {
                return Err(CounterSampleError::BackwardDiscontinuity);
            }
            WrapBehavior::Reject => self.extended + (raw - previous),
            WrapBehavior::ExtendModulo if raw >= previous => self.extended + (raw - previous),
            WrapBehavior::ExtendModulo => {
                let backwards = previous - raw;
                if backwards <= modulus / 2 {
                    return Err(CounterSampleError::BackwardDiscontinuity);
                }
                self.extended + (modulus - previous) + raw
            }
        };
        self.previous_raw = Some(raw as u64);
        self.extended = next;
        u64::try_from(next).map_err(|_| CounterSampleError::ExtendedValueOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::{CounterExtender, CounterSampleError};
    use crate::target::{
        CalibrationPolicy, ClockUnit, DiscontinuityPolicy, Initialization, InterruptSafety,
        ReadAtomicity, ResetPolicy, SleepPolicy, TimeProviderContract, TimeProviderManifest,
        WrapBehavior,
    };

    fn provider(width: u8, wrap_behavior: WrapBehavior) -> TimeProviderContract {
        TimeProviderContract::validate(TimeProviderManifest {
            name: "fake".to_owned(),
            read_symbol: "fake_read".to_owned(),
            clock_unit: ClockUnit::Ticks,
            counter_width: width,
            wrap_behavior,
            frequency_hz: 1_000_000,
            read_atomicity: ReadAtomicity::SingleWord,
            interrupt_safety: InterruptSafety::Safe,
            initialization: Initialization::ReadyAtEntry,
            calibration: CalibrationPolicy::Fixed,
            sleep: SleepPolicy::Continues,
            reset: ResetPolicy::Resets,
            discontinuity: DiscontinuityPolicy::Reject,
        })
        .expect("fake provider should validate")
    }

    #[test]
    fn fake_provider_extends_one_wrap_and_rejects_ambiguous_backwards_steps() {
        let mut counter = CounterExtender::new(&provider(8, WrapBehavior::ExtendModulo));
        assert_eq!(counter.observe(250), Ok(250));
        assert_eq!(counter.observe(2), Ok(258));
        assert_eq!(counter.observe(1), Err(CounterSampleError::BackwardDiscontinuity));
    }

    #[test]
    fn reject_provider_does_not_silently_wrap() {
        let mut counter = CounterExtender::new(&provider(8, WrapBehavior::Reject));
        assert_eq!(counter.observe(250), Ok(250));
        assert_eq!(counter.observe(2), Err(CounterSampleError::BackwardDiscontinuity));
    }
}
