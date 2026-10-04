use std::cell::Cell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static MONOTONIC_ORIGIN: OnceLock<Instant> = OnceLock::new();

const SLEEP_CONTEXT_INTERRUPT: u32 = 1;
const SLEEP_CONTEXT_CRITICAL_SECTION: u32 = 2;
const SLEEP_CONTEXT_MASK: u32 = SLEEP_CONTEXT_INTERRUPT | SLEEP_CONTEXT_CRITICAL_SECTION;

#[derive(Clone, Copy, Default)]
struct SleepContextState {
    interrupt_depth: u32,
    critical_section_depth: u32,
}

thread_local! {
    static SLEEP_CONTEXT_STATE: Cell<SleepContextState> = const { Cell::new(SleepContextState {
        interrupt_depth: 0,
        critical_section_depth: 0,
    }) };
}

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

/// Sleeps the current hosted execution context for a monotonic duration.
///
/// The zero status is success; `-1` means the selected runtime cannot provide
/// scheduler-aware sleeping or the current execution context forbids blocking.
#[unsafe(no_mangle)]
pub extern "C" fn actus_sleep_nanos(nanos: u64) -> i32 {
    if sleep_is_forbidden() {
        return -1;
    }
    std::thread::sleep(Duration::from_nanos(nanos));
    0
}

/// Enter an execution context in which scheduler-aware blocking is forbidden.
///
/// Target providers call this hook when entering an interrupt handler or a
/// critical section. The state is thread-local and nestable. Unknown flags or
/// counter overflow return `-1`; a successful update returns `0`.
#[unsafe(no_mangle)]
pub extern "C" fn actus_sleep_context_enter(flags: u32) -> i32 {
    if flags == 0 || flags & !SLEEP_CONTEXT_MASK != 0 {
        return -1;
    }
    SLEEP_CONTEXT_STATE.with(|state| {
        let current = state.get();
        let interrupt_depth = if flags & SLEEP_CONTEXT_INTERRUPT != 0 {
            current.interrupt_depth.checked_add(1)
        } else {
            Some(current.interrupt_depth)
        };
        let critical_section_depth = if flags & SLEEP_CONTEXT_CRITICAL_SECTION != 0 {
            current.critical_section_depth.checked_add(1)
        } else {
            Some(current.critical_section_depth)
        };
        match (interrupt_depth, critical_section_depth) {
            (Some(interrupt_depth), Some(critical_section_depth)) => {
                state.set(SleepContextState { interrupt_depth, critical_section_depth });
                0
            }
            _ => -1,
        }
    })
}

/// Leave a previously entered restricted execution context.
///
/// The flags must have a matching active entry on the current execution
/// context. Mismatched exits return `-1`; a successful update returns `0`.
#[unsafe(no_mangle)]
pub extern "C" fn actus_sleep_context_exit(flags: u32) -> i32 {
    if flags == 0 || flags & !SLEEP_CONTEXT_MASK != 0 {
        return -1;
    }
    SLEEP_CONTEXT_STATE.with(|state| {
        let current = state.get();
        if (flags & SLEEP_CONTEXT_INTERRUPT != 0 && current.interrupt_depth == 0)
            || (flags & SLEEP_CONTEXT_CRITICAL_SECTION != 0 && current.critical_section_depth == 0)
        {
            return -1;
        }
        state.set(SleepContextState {
            interrupt_depth: if flags & SLEEP_CONTEXT_INTERRUPT != 0 {
                current.interrupt_depth - 1
            } else {
                current.interrupt_depth
            },
            critical_section_depth: if flags & SLEEP_CONTEXT_CRITICAL_SECTION != 0 {
                current.critical_section_depth - 1
            } else {
                current.critical_section_depth
            },
        });
        0
    })
}

fn sleep_is_forbidden() -> bool {
    SLEEP_CONTEXT_STATE.with(|state| {
        let state = state.get();
        state.interrupt_depth != 0 || state.critical_section_depth != 0
    })
}

fn checked_nanoseconds(duration: Duration) -> Option<u64> {
    u64::try_from(duration.as_nanos()).ok()
}

#[cfg(test)]
mod tests {
    use super::{
        SLEEP_CONTEXT_CRITICAL_SECTION, SLEEP_CONTEXT_INTERRUPT, actus_monotonic_nanos,
        actus_sleep_context_enter, actus_sleep_context_exit, actus_sleep_nanos,
        checked_nanoseconds,
    };
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

    #[test]
    fn blocking_sleep_is_rejected_in_restricted_contexts() {
        assert_eq!(actus_sleep_context_enter(SLEEP_CONTEXT_INTERRUPT), 0);
        assert_eq!(actus_sleep_nanos(0), -1);
        assert_eq!(actus_sleep_context_enter(SLEEP_CONTEXT_INTERRUPT), 0);
        assert_eq!(actus_sleep_context_exit(SLEEP_CONTEXT_INTERRUPT), 0);
        assert_eq!(actus_sleep_nanos(0), -1);
        assert_eq!(actus_sleep_context_exit(SLEEP_CONTEXT_INTERRUPT), 0);
        assert_eq!(actus_sleep_nanos(0), 0);

        assert_eq!(actus_sleep_context_enter(SLEEP_CONTEXT_CRITICAL_SECTION), 0);
        assert_eq!(actus_sleep_nanos(0), -1);
        assert_eq!(actus_sleep_context_exit(SLEEP_CONTEXT_CRITICAL_SECTION), 0);
        assert_eq!(actus_sleep_nanos(0), 0);
    }

    #[test]
    fn invalid_context_flags_and_unbalanced_exit_fail() {
        assert_eq!(actus_sleep_context_enter(0), -1);
        assert_eq!(actus_sleep_context_enter(4), -1);
        assert_eq!(actus_sleep_context_exit(SLEEP_CONTEXT_INTERRUPT), -1);
    }
}
