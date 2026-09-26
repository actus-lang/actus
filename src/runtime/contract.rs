/// Version of the C ABI exported by the hosted Actus runtime.
pub const RUNTIME_ABI_VERSION: u32 = 1;

/// Stable runtime symbol names used by native lowering.
pub const BUFFER_ALLOCATE_SYMBOL: &str = "actus_buffer_allocate";
pub const BUFFER_DROP_SYMBOL: &str = "actus_buffer_drop";
pub const BUFFER_APPEND_SYMBOL: &str = "actus_buffer_append";
pub const PRINT_INT_SYMBOL: &str = "actus_print_int";
pub const PRINT_STRING_SYMBOL: &str = "actus_print_string";
pub const PRINT_INT_STDERR_SYMBOL: &str = "actus_print_int_stderr";
pub const PRINT_STRING_STDERR_SYMBOL: &str = "actus_print_string_stderr";
pub const PRINT_BUFFER_STDOUT_SYMBOL: &str = "actus_print_buffer_stdout";
pub const PRINT_BUFFER_STDERR_SYMBOL: &str = "actus_print_buffer_stderr";
pub const FLUSH_STDOUT_SYMBOL: &str = "actus_flush_stdout";
pub const PRINT_LINE_BUFFER_STDOUT_SYMBOL: &str = "actus_print_line_buffer_stdout";
pub const PRINT_LINE_BUFFER_STDERR_SYMBOL: &str = "actus_print_line_buffer_stderr";
pub const READ_STDIN_LINE_SYMBOL: &str = "actus_read_stdin_line";
pub const READ_BYTE_SYMBOL: &str = "actus_read_byte";

/// Runtime operations are explicit capabilities rather than implicit compiler
/// services. Freestanding targets may provide none of these capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCapability {
    Buffer,
    Stdout,
}
