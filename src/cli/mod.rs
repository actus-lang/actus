mod build;
mod check;
mod conformance;
mod diagnostics;
mod dispatch;
mod fmt;
mod input;
mod lock;
mod lsp;
mod project;
mod publish;
mod run;
mod test_runner;
mod version;
mod watch;

pub use dispatch::{run, run_with_args};
