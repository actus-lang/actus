pub(super) fn lsp_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    if arguments.next().is_some() {
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
