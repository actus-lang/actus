use std::fmt::{Display, Formatter};

use serde::Deserialize;

/// Manifest representation of a monotonic clock provider.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TimeProviderManifest {
    pub name: String,
    pub read_symbol: String,
    pub clock_unit: ClockUnit,
    pub counter_width: u8,
    pub wrap_behavior: WrapBehavior,
    pub frequency_hz: u64,
    pub read_atomicity: ReadAtomicity,
    pub interrupt_safety: InterruptSafety,
    pub initialization: Initialization,
    pub calibration: CalibrationPolicy,
    pub sleep: SleepPolicy,
    pub reset: ResetPolicy,
    pub discontinuity: DiscontinuityPolicy,
}

/// Unit returned by a provider before conversion to the public `Instant` domain.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ClockUnit {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Ticks,
}

/// Counter overflow behavior is explicit because implicit wrapping is unsafe.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WrapBehavior {
    ExtendModulo,
    Reject,
}

/// Atomicity contract for one provider read.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReadAtomicity {
    SingleWord,
    InterruptMasked,
    Seqlock,
}

/// Whether a provider read is valid while servicing an interrupt.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InterruptSafety {
    Safe,
    Forbidden,
}

/// Provider initialization boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Initialization {
    ReadyAtEntry,
    RequiresInit,
    ExplicitInit,
}

/// Whether the provider frequency may change after initialization.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CalibrationPolicy {
    Fixed,
    MayChange,
}

/// Counter behavior while the target is asleep.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SleepPolicy {
    Continues,
    Pauses,
    Unknown,
}

/// Counter behavior across a target reset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResetPolicy {
    Resets,
    Preserves,
}

/// Required behavior when a provider reports a discontinuous sample.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiscontinuityPolicy {
    Reject,
    ClampForward,
}

/// Validated, target-neutral provider contract used by configuration and cache identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeProviderContract {
    manifest: TimeProviderManifest,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ProviderContractError(String);

impl Display for ProviderContractError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ProviderContractError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderConversionError {
    Overflow,
}

impl Display for ProviderConversionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("time provider sample cannot be represented as u64 nanoseconds")
    }
}

impl std::error::Error for ProviderConversionError {}

impl TimeProviderContract {
    pub fn validate(manifest: TimeProviderManifest) -> Result<Self, ProviderContractError> {
        if manifest.name.trim().is_empty() {
            return Err(ProviderContractError("time provider name must not be empty".to_owned()));
        }
        if !is_symbol_name(&manifest.read_symbol) {
            return Err(ProviderContractError(format!(
                "time provider `{}` read_symbol must be a non-empty C symbol name",
                manifest.name
            )));
        }
        if !matches!(manifest.counter_width, 8 | 16 | 32 | 64) {
            return Err(ProviderContractError(format!(
                "time provider `{}` counter_width must be one of 8, 16, 32, or 64",
                manifest.name
            )));
        }
        if manifest.frequency_hz == 0 {
            return Err(ProviderContractError(format!(
                "time provider `{}` frequency_hz must be positive",
                manifest.name
            )));
        }
        let expected_frequency = match manifest.clock_unit {
            ClockUnit::Nanoseconds => Some(1_000_000_000),
            ClockUnit::Microseconds => Some(1_000_000),
            ClockUnit::Milliseconds => Some(1_000),
            ClockUnit::Ticks => None,
        };
        if expected_frequency.is_some_and(|expected| manifest.frequency_hz != expected) {
            return Err(ProviderContractError(format!(
                "time provider `{}` frequency_hz does not match clock_unit",
                manifest.name
            )));
        }
        if manifest.wrap_behavior == WrapBehavior::ExtendModulo && manifest.counter_width > 64 {
            return Err(ProviderContractError(format!(
                "time provider `{}` cannot extend a counter wider than 64 bits",
                manifest.name
            )));
        }
        if manifest.calibration == CalibrationPolicy::MayChange
            && manifest.discontinuity == DiscontinuityPolicy::ClampForward
        {
            return Err(ProviderContractError(format!(
                "time provider `{}` cannot clamp calibration changes without a fixed frequency",
                manifest.name
            )));
        }
        if manifest.sleep == SleepPolicy::Unknown
            && manifest.discontinuity == DiscontinuityPolicy::ClampForward
        {
            return Err(ProviderContractError(format!(
                "time provider `{}` must define sleep behavior before clamping discontinuities",
                manifest.name
            )));
        }
        Ok(Self { manifest })
    }

    pub fn hosted_default() -> Self {
        Self {
            manifest: TimeProviderManifest {
                name: "host-monotonic".to_owned(),
                read_symbol: "actus_monotonic_nanos".to_owned(),
                clock_unit: ClockUnit::Nanoseconds,
                counter_width: 64,
                wrap_behavior: WrapBehavior::Reject,
                frequency_hz: 1_000_000_000,
                read_atomicity: ReadAtomicity::SingleWord,
                interrupt_safety: InterruptSafety::Safe,
                initialization: Initialization::ReadyAtEntry,
                calibration: CalibrationPolicy::Fixed,
                sleep: SleepPolicy::Continues,
                reset: ResetPolicy::Resets,
                discontinuity: DiscontinuityPolicy::Reject,
            },
        }
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }
    pub fn read_symbol(&self) -> &str {
        &self.manifest.read_symbol
    }
    pub const fn clock_unit(&self) -> ClockUnit {
        self.manifest.clock_unit
    }
    pub const fn counter_width(&self) -> u8 {
        self.manifest.counter_width
    }
    pub const fn wrap_behavior(&self) -> WrapBehavior {
        self.manifest.wrap_behavior
    }
    pub const fn frequency_hz(&self) -> u64 {
        self.manifest.frequency_hz
    }
    pub const fn read_atomicity(&self) -> ReadAtomicity {
        self.manifest.read_atomicity
    }
    pub const fn interrupt_safety(&self) -> InterruptSafety {
        self.manifest.interrupt_safety
    }
    pub const fn initialization(&self) -> Initialization {
        self.manifest.initialization
    }
    pub const fn calibration(&self) -> CalibrationPolicy {
        self.manifest.calibration
    }
    pub const fn sleep(&self) -> SleepPolicy {
        self.manifest.sleep
    }
    pub const fn reset(&self) -> ResetPolicy {
        self.manifest.reset
    }
    pub const fn discontinuity(&self) -> DiscontinuityPolicy {
        self.manifest.discontinuity
    }

    /// Converts one provider sample into public nanoseconds using checked integer arithmetic.
    pub fn ticks_to_nanos(&self, ticks: u64) -> Result<u64, ProviderConversionError> {
        let nanos = match self.clock_unit() {
            ClockUnit::Nanoseconds => u128::from(ticks),
            ClockUnit::Microseconds => u128::from(ticks) * 1_000,
            ClockUnit::Milliseconds => u128::from(ticks) * 1_000_000,
            ClockUnit::Ticks => {
                u128::from(ticks)
                    .checked_mul(1_000_000_000)
                    .ok_or(ProviderConversionError::Overflow)?
                    / u128::from(self.frequency_hz())
            }
        };
        u64::try_from(nanos).map_err(|_| ProviderConversionError::Overflow)
    }

    pub fn spec_hash(&self, target_hash: &str) -> String {
        let identity = format!(
            "{target_hash};name={};symbol={};unit={:?};width={};wrap={:?};frequency={};atomicity={:?};interrupt={:?};init={:?};calibration={:?};sleep={:?};reset={:?};discontinuity={:?}",
            self.name(),
            self.read_symbol(),
            self.clock_unit(),
            self.counter_width(),
            self.wrap_behavior(),
            self.frequency_hz(),
            self.read_atomicity(),
            self.interrupt_safety(),
            self.initialization(),
            self.calibration(),
            self.sleep(),
            self.reset(),
            self.discontinuity()
        );
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in identity.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }
}

fn is_symbol_name(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
