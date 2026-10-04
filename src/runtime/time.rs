use std::sync::OnceLock;
use std::time::{Duration, Instant};

static MONOTONIC_ORIGIN: OnceLock<Instant> = OnceLock::new();

/// Returns elapsed monotonic nanoseconds from the hosted runtime origin.
///
/// The origin is process-local and intentionally not wall-clock time. The
/// conversion is checked; an unrepresentable value is a hard runtime failure
/// rather than a wrapped or truncated timestamp.
#[unsafe(no_mangle)]
pub extern "C" fn actus_monotonic_nanos() -> u64 {
    let origin = MONOTONIC_ORIGIN.get_or_init(Instant::now);
    match checked_nanoseconds(origin.elapsed()) {
        Some(value) => value,
        None => std::process::abort(),
    }
}

fn checked_nanoseconds(duration: Duration) -> Option<u64> {
    u64::try_from(duration.as_nanos()).ok()
}

#[cfg(test)]
mod tests {
    use super::{actus_monotonic_nanos, checked_nanoseconds};
    use std::time::Duration;

    #[test]
    fn sequential_reads_are_non_decreasing() {
        let first = actus_monotonic_nanos();
        let second = actus_monotonic_nanos();
        assert!(second >= first, "monotonic clock moved backwards");
    }

    #[test]
    fn nanosecond_conversion_rejects_u64_overflow() {
        let duration = Duration::new(u64::MAX, 999_999_999);
        assert_eq!(checked_nanoseconds(duration), None);
    }
}
