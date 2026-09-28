use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FunctionRange {
    pub(super) name: String,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
}

pub(super) fn find_functions(source: &str) -> Vec<FunctionRange> {
    FunctionScanner::scan(source)
}

#[derive(Default)]
struct FunctionScanner {
    functions: Vec<FunctionRange>,
    active: Option<(usize, String)>,
    depth: usize,
    body_started: bool,
}

impl FunctionScanner {
    fn scan(source: &str) -> Vec<FunctionRange> {
        let lines = source.lines().collect::<Vec<_>>();
        let mut scanner = Self::default();
        let mut in_block_comment = false;
        for (index, line) in lines.iter().enumerate() {
            scanner.scan_line(index, line, &mut in_block_comment);
        }
        scanner.finish(lines.len());
        scanner.functions
    }

    fn scan_line(&mut self, index: usize, line: &str, in_block_comment: &mut bool) {
        let cleaned = strip_non_code(line, in_block_comment);
        if self.active.is_none()
            && let Some(name) = declaration_name(&cleaned)
        {
            self.active = Some((index + 1, name));
            self.depth = 0;
            self.body_started = false;
        }
        let Some((start_line, name)) = &self.active else { return };
        let (opens, closes) = brace_counts(&cleaned);
        self.body_started |= opens > 0;
        self.depth = self.depth.saturating_add(opens).saturating_sub(closes);
        if self.body_started && self.depth == 0 {
            self.functions.push(FunctionRange {
                name: name.clone(),
                start_line: *start_line,
                end_line: index + 1,
            });
            self.active = None;
        } else if !self.body_started && cleaned.contains(';') {
            self.active = None;
        }
    }

    fn finish(&mut self, line_count: usize) {
        if let Some((start_line, name)) = self.active.take() {
            self.functions.push(FunctionRange { name, start_line, end_line: line_count });
        }
    }
}

pub(super) fn line_span(source: &str, start_line: usize, end_line: usize) -> SourceSpan {
    let starts = line_starts(source);
    let start = starts.get(start_line.saturating_sub(1)).copied().unwrap_or(0);
    let end_index = end_line.min(starts.len()).saturating_sub(1);
    let end = starts.get(end_index + 1).copied().unwrap_or(source.len());
    SourceSpan::new(start, end)
}

fn declaration_name(line: &str) -> Option<String> {
    let words = line.split_whitespace().collect::<Vec<_>>();
    let index = words.iter().position(|word| *word == "fn" || *word == "verb")?;
    let name = words.get(index + 1)?;
    let name = name.split(['(', '<', ':', '[']).next().unwrap_or(name);
    (!name.is_empty()).then_some(name.to_owned())
}

fn brace_counts(line: &str) -> (usize, usize) {
    (
        line.bytes().filter(|byte| *byte == b'{').count(),
        line.bytes().filter(|byte| *byte == b'}').count(),
    )
}

fn strip_non_code(line: &str, in_block_comment: &mut bool) -> String {
    let mut output = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let characters = line.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        let next = characters.get(index + 1).copied();
        if let Some(next_index) = consume_block_comment(&characters, index, in_block_comment) {
            index = next_index;
            continue;
        }
        if !in_string && character == '/' && next == Some('/') {
            break;
        }
        if !in_string && character == '/' && next == Some('*') {
            *in_block_comment = true;
            index += 2;
            continue;
        }
        append_visible_character(character, &mut in_string, &mut escaped, &mut output);
        index += 1;
    }
    output
}

fn append_visible_character(
    character: char,
    in_string: &mut bool,
    escaped: &mut bool,
    output: &mut String,
) {
    if character == '"' && !*escaped {
        *in_string = !*in_string;
    } else if !*in_string {
        output.push(character);
    }
    *escaped = character == '\\' && !*escaped;
    if character != '\\' {
        *escaped = false;
    }
}

fn consume_block_comment(
    characters: &[char],
    index: usize,
    in_block_comment: &mut bool,
) -> Option<usize> {
    if !*in_block_comment {
        return None;
    }
    if characters[index] == '*' && characters.get(index + 1) == Some(&'/') {
        *in_block_comment = false;
        Some(index + 2)
    } else {
        Some(index + 1)
    }
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, character) in source.char_indices() {
        if character == '\n' {
            starts.push(index + 1);
        }
    }
    starts
}
