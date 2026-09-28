//! Strict validation for public Actus documentation contracts.
//!
//! Validation operates on parsed declarations so nested members cannot evade
//! the public API documentation gate.

mod contract;
mod language;
mod model;
mod rules;

pub use model::{DocumentationIssue, DocumentationSection};
pub use rules::validate_public_documentation;
