use super::Scanner;

impl<'source> Scanner<'source> {
    pub(super) fn match_character(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor += character.len_utf8();
        Some(character)
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    pub(super) fn peek_next(&self) -> Option<char> {
        self.source[self.cursor..].chars().nth(1)
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.cursor >= self.source.len()
    }
}
