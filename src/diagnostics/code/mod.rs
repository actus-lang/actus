//! The diagnostic code registry: one [`DiagnosticCode`] constant per code.

mod registry;

#[cfg(test)]
mod tests;

pub use registry::DiagnosticCode;
