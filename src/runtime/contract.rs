/// Version of the C ABI exported by the hosted Actus runtime.
pub const RUNTIME_ABI_VERSION: u32 = 1;

/// Successful scalar C-ABI operation status.
pub const ABI_STATUS_SUCCESS: i32 = 0;

/// Failed scalar C-ABI operation status.
pub const ABI_STATUS_FAILURE: i32 = -1;

/// End-of-stream status returned by byte-oriented input bridges.
pub const ABI_STATUS_END_OF_STREAM: i32 = -2;

/// Failed opaque-handle C-ABI result.
pub const ABI_HANDLE_FAILURE: i64 = -1;

/// Returns whether a scalar status is a non-negative byte count or position.
pub const fn is_successful_count(status: i32) -> bool {
    status >= ABI_STATUS_SUCCESS
}

/// Stable runtime symbol names used by native lowering.
pub const BUFFER_ALLOCATE_SYMBOL: &str = "actus_buffer_allocate";
pub const BUFFER_DROP_SYMBOL: &str = "actus_buffer_drop";
pub const BUFFER_APPEND_SYMBOL: &str = "actus_buffer_append";
pub const PRINT_INT_SYMBOL: &str = "actus_print_int";
pub const PRINT_STRING_SYMBOL: &str = "actus_print_string";
pub const WRITE_STRING_STDOUT_SYMBOL: &str = "actus_write_string_stdout";
/// Stable runtime symbol for checked UTF-8 byte-length inspection.
pub const STRING_LENGTH_SYMBOL: &str = "actus_string_length";
/// Stable runtime symbol for checked UTF-8 byte inspection.
pub const STRING_BYTE_AT_SYMBOL: &str = "actus_string_byte_at";
/// String pointer was null at the ABI boundary.
pub const STRING_STATUS_NULL: i32 = -1;
/// String bytes were not valid UTF-8.
pub const STRING_STATUS_INVALID_UTF8: i32 = -2;
/// The requested byte index was outside the validated string.
pub const STRING_STATUS_OUT_OF_BOUNDS: i32 = -3;
pub const PRINT_INT_STDERR_SYMBOL: &str = "actus_print_int_stderr";
pub const PRINT_STRING_STDERR_SYMBOL: &str = "actus_print_string_stderr";
pub const PRINT_BUFFER_STDOUT_SYMBOL: &str = "actus_print_buffer_stdout";
pub const PRINT_BUFFER_STDERR_SYMBOL: &str = "actus_print_buffer_stderr";
pub const FLUSH_STDOUT_SYMBOL: &str = "actus_flush_stdout";
pub const FLUSH_STDERR_SYMBOL: &str = "actus_flush_stderr";
pub const PRINT_LINE_BUFFER_STDOUT_SYMBOL: &str = "actus_print_line_buffer_stdout";
pub const PRINT_LINE_BUFFER_STDERR_SYMBOL: &str = "actus_print_line_buffer_stderr";
pub const READ_STDIN_LINE_SYMBOL: &str = "actus_read_stdin_line";
pub const READ_BYTE_SYMBOL: &str = "actus_read_byte";
pub const WRITE_BUFFER_STDOUT_SYMBOL: &str = "actus_write_buffer_stdout";
pub const BUFFER_RESERVE_SYMBOL: &str = "actus_buffer_reserve";
pub const BUFFERED_WRITE_STDOUT_SYMBOL: &str = "actus_buffered_write_stdout";
pub const FLUSH_BUFFERED_STDOUT_SYMBOL: &str = "actus_flush_buffered_stdout";
pub const ENUM_ALLOCATE_SYMBOL: &str = "actus_enum_allocate";
pub const ENUM_DROP_SYMBOL: &str = "actus_enum_drop";
/// Stable hosted runtime symbol for monotonic nanosecond reads.
pub const MONOTONIC_NANOS_SYMBOL: &str = "actus_monotonic_nanos";
/// Stable hosted runtime symbol for scheduler-aware monotonic sleeping.
pub const SLEEP_NANOS_SYMBOL: &str = "actus_sleep_nanos";
/// Stable runtime symbol used by a target provider to enter a restricted
/// execution context for blocking operations.
pub const SLEEP_CONTEXT_ENTER_SYMBOL: &str = "actus_sleep_context_enter";
/// Stable runtime symbol used by a target provider to leave a restricted
/// execution context for blocking operations.
pub const SLEEP_CONTEXT_EXIT_SYMBOL: &str = "actus_sleep_context_exit";
/// Execution-context flag indicating interrupt context.
pub const SLEEP_CONTEXT_INTERRUPT: u32 = 1;
/// Execution-context flag indicating a critical section.
pub const SLEEP_CONTEXT_CRITICAL_SECTION: u32 = 2;

/// Runtime operations are explicit capabilities rather than implicit compiler
/// services. Freestanding targets may provide none of these capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCapability {
    Buffer,
    Stdout,
}
