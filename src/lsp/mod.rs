mod completion;
mod definition;
mod diagnostics;
mod documents;
mod formatting;
mod hover;
mod module_scope;
mod position;
mod protocol;
mod semantic_tokens;
mod server;

pub use diagnostics::analyze_document;
pub use server::run_stdio;
