use std::io::{self, IsTerminal};

pub(super) struct TestOutputStyle {
    enabled: bool,
}

impl TestOutputStyle {
    pub(super) fn detect() -> Self {
        Self { enabled: io::stdout().is_terminal() }
    }

    pub(super) fn success(&self, text: &str) -> String {
        self.paint("\x1b[32m", text)
    }

    pub(super) fn failure(&self, text: &str) -> String {
        self.paint("\x1b[31m", text)
    }

    fn paint(&self, color: &str, text: &str) -> String {
        if self.enabled { format!("{color}{text}\x1b[0m") } else { text.to_owned() }
    }
}
