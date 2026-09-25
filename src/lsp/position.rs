use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Clone, Debug)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (offset, character) in source.char_indices() {
            if character == '\n' {
                starts.push(offset + character.len_utf8());
            }
        }
        Self { starts }
    }

    pub fn position(&self, source: &str, offset: usize) -> LspPosition {
        let offset = offset.min(source.len());
        let line = self.line_for(offset);
        let line_start = self.starts[line];
        let character = source[line_start..offset].encode_utf16().count();
        LspPosition { line: line as u32, character: character as u32 }
    }

    pub fn byte_offset(&self, source: &str, position: &LspPosition) -> Option<usize> {
        let line = usize::try_from(position.line).ok()?;
        let character = usize::try_from(position.character).ok()?;
        let line_start = *self.starts.get(line)?;
        let line_end = self.starts.get(line + 1).copied().unwrap_or(source.len());
        let line_text_end = line_content_end(source, line_start, line_end);
        let line_text = &source[line_start..line_text_end];
        let mut utf16_count = 0;
        for (offset, character_value) in line_text.char_indices() {
            if utf16_count == character {
                return Some(line_start + offset);
            }
            utf16_count += character_value.len_utf16();
        }
        (utf16_count == character).then_some(line_text_end)
    }

    fn line_for(&self, offset: usize) -> usize {
        self.starts.partition_point(|start| *start <= offset).saturating_sub(1)
    }
}

fn line_content_end(source: &str, line_start: usize, line_end: usize) -> usize {
    let mut content_end = line_end;
    if content_end > line_start && source.as_bytes().get(content_end - 1) == Some(&b'\n') {
        content_end -= 1;
        if content_end > line_start && source.as_bytes().get(content_end - 1) == Some(&b'\r') {
            content_end -= 1;
        }
    }
    content_end
}

pub fn byte_range(source: &str, range: &LspRange) -> Option<std::ops::Range<usize>> {
    let index = LineIndex::new(source);
    let start = index.byte_offset(source, &range.start)?;
    let end = index.byte_offset(source, &range.end)?;
    (start <= end).then_some(start..end)
}

#[cfg(test)]
mod tests {
    use super::{LspPosition, LspRange, byte_range};

    #[test]
    fn accepts_scalar_boundaries_and_rejects_invalid_utf16_offsets() {
        let source = "აბგ";
        let range = LspRange {
            start: LspPosition { line: 0, character: 1 },
            end: LspPosition { line: 0, character: 2 },
        };
        assert!(byte_range(source, &range).is_some());
        let split = LspRange {
            start: LspPosition { line: 0, character: 1 },
            end: LspPosition { line: 0, character: 1 },
        };
        assert_eq!(byte_range(source, &split), Some(3..3));
        let invalid_utf16 = LspRange {
            start: LspPosition { line: 0, character: 3 },
            end: LspPosition { line: 0, character: 3 },
        };
        assert_eq!(byte_range("🚀", &invalid_utf16), None);
    }

    #[test]
    fn rejects_positions_past_line_content() {
        let range = LspRange {
            start: LspPosition { line: 0, character: 3 },
            end: LspPosition { line: 0, character: 3 },
        };
        assert_eq!(byte_range("ab\ncd", &range), None);
    }
}
