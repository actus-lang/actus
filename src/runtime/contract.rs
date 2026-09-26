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

/// Runtime operations are explicit capabilities rather than implicit compiler
/// services. Freestanding targets may provide none of these capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCapability {
    Buffer,
    Stdout,
}
