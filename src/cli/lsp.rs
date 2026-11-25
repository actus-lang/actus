use crate::cli::terminal_output::errln;

pub(super) fn lsp_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    if let Some(argument) = arguments.next() {
        if super::input::is_option_like(&argument) {
            errln!("error: {}", super::input::argument_error("lsp", &argument));
            return 2;
        }
        errln!("error: `actus lsp` does not accept arguments");
        return 2;
    }
    match crate::lsp::run_stdio() {
        Ok(()) => 0,
        // The client closing the connection is how a session ends.
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
            super::terminal_output::CLOSED_STREAM_STATUS
        }
        Err(error) => {
            errln!("error: lsp server failed: {error}");
            1
        }
    }
}
