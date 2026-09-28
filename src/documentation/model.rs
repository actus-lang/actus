use crate::lexer::SourceSpan;

/// Stable public-documentation contract sections reported by strict checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentationSection {
    /// Human-readable purpose and behavior.
    Purpose,
    /// Parameter ownership and borrowing roles.
    Ownership,
    /// Returned values and their variants.
    Returns,
    /// Failure and error behavior.
    Errors,
    /// Mutation and state-transition behavior.
    Mutation,
    /// Allocation, storage, and zero-allocation guarantees.
    Allocation,
    /// Lifetime, cleanup, and resource-release behavior.
    Cleanup,
    /// Native ABI or runtime bridge behavior.
    Abi,
    /// Target and platform-specific behavior.
    Platform,
}

impl DocumentationSection {
    /// Returns the stable contract label used in diagnostics and fixtures.
    pub fn label(self) -> &'static str {
        match self {
            Self::Purpose => "purpose",
            Self::Ownership => "ownership",
            Self::Returns => "returns",
            Self::Errors => "errors",
            Self::Mutation => "mutation",
            Self::Allocation => "allocation",
            Self::Cleanup => "cleanup",
            Self::Abi => "ABI",
            Self::Platform => "platform",
        }
    }
}

/// One deterministic failure in a public Actus documentation contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentationIssue {
    /// Stable strict-conformance code in the reserved E184x range.
    pub code: &'static str,
    /// Declaration or member whose documentation failed validation.
    pub declaration: String,
    /// Contract section that must be corrected.
    pub section: DocumentationSection,
    /// Source span of the declaration, not the unrelated docstring token.
    pub span: SourceSpan,
    /// Review-oriented explanation of the rejected contract.
    pub message: String,
}
