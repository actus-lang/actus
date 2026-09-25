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
        let line_text = &source[line_start..line_end];
        let mut utf16_count = 0;
        for (offset, character_value) in line_text.char_indices() {
            if utf16_count == character {
                return Some(line_start + offset);
            }
            utf16_count += character_value.len_utf16();
        }
        (utf16_count == character).then_some(line_end)
    }

    fn line_for(&self, offset: usize) -> usize {
        self.starts.partition_point(|start| *start <= offset).saturating_sub(1)
    }
}

pub fn byte_range(source: &str, range: &LspRange) -> Option<std::ops::Range<usize>> {
    let index = LineIndex::new(source);
    let start = index.byte_offset(source, &range.start)?;
    let end = index.byte_offset(source, &range.end)?;
    (start <= end).then_some(start..end)
}
