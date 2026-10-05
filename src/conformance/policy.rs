/// The source-size thresholds enforced by strict conformance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLimitPolicy {
    /// The preferred maximum number of code-bearing lines in one source file.
    pub preferred_file_lines: usize,
    /// The code-line count at which a source file must be split.
    pub file_split_lines: usize,
    /// The hard maximum number of code-bearing lines in one source file.
    pub hard_file_lines: usize,
    /// The preferred maximum number of code-bearing lines in one function.
    pub preferred_function_lines: usize,
    /// The code-line count at which a function must be split.
    pub function_split_lines: usize,
    /// The hard maximum number of code-bearing lines in one function.
    pub hard_function_lines: usize,
}

impl Default for SourceLimitPolicy {
    fn default() -> Self {
        Self {
            preferred_file_lines: 300,
            file_split_lines: 400,
            hard_file_lines: 500,
            preferred_function_lines: 30,
            function_split_lines: 40,
            hard_function_lines: 60,
        }
    }
}
