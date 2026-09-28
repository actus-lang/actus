use super::{ComponentRange, WINDOWS_BACKSLASH, WINDOWS_SEPARATOR};

pub fn components<'a>(units: &'a [u16], windows: bool) -> Components<'a> {
    Components { units, windows, cursor: 0 }
}

pub fn file_name(units: &[u16], windows: bool) -> Option<ComponentRange> {
    components(units, windows).last()
}

pub fn file_stem(units: &[u16], windows: bool) -> Option<ComponentRange> {
    let file = file_name(units, windows)?;
    let dot =
        units[file.start..file.end].iter().rposition(|unit| *unit == b'.' as u16)? + file.start;
    (dot > file.start).then_some(ComponentRange { start: file.start, end: dot })
}

pub fn extension(units: &[u16], windows: bool) -> Option<ComponentRange> {
    let file = file_name(units, windows)?;
    let dot =
        units[file.start..file.end].iter().rposition(|unit| *unit == b'.' as u16)? + file.start;
    (dot > file.start && dot + 1 < file.end)
        .then_some(ComponentRange { start: dot + 1, end: file.end })
}

pub struct Components<'a> {
    units: &'a [u16],
    windows: bool,
    cursor: usize,
}

impl<'a> Iterator for Components<'a> {
    type Item = ComponentRange;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.units.len() && self.is_separator(self.units[self.cursor]) {
            self.cursor += 1;
        }
        let start = self.cursor;
        while self.cursor < self.units.len() && !self.is_separator(self.units[self.cursor]) {
            self.cursor += 1;
        }
        (start < self.cursor).then_some(ComponentRange { start, end: self.cursor })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.units.len().saturating_sub(self.cursor)))
    }
}

impl Components<'_> {
    fn is_separator(&self, unit: u16) -> bool {
        unit == WINDOWS_SEPARATOR || (self.windows && unit == WINDOWS_BACKSLASH)
    }
}
