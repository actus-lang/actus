pub(super) fn lsp_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    if let Some(argument) = arguments.next() {
        if super::input::is_option_like(&argument) {
            eprintln!("error: {}", super::input::argument_error("lsp", &argument));
            return 2;
        }
        eprintln!("error: `actus lsp` does not accept arguments");
        return 2;
    }
    match crate::lsp::run_stdio() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: lsp server failed: {error}");
            1
        }
    }
}
