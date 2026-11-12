mod editor;
mod protocol;
mod queries;
mod workspace;

pub use editor::diagnostics::analyze_document;
pub use protocol::server::run_stdio;
