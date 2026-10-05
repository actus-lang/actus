use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FunctionRange {
    pub(super) name: String,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
}

pub(super) fn find_functions(source: &str, hash_comments: bool) -> Vec<FunctionRange> {
    FunctionScanner::scan(source, hash_comments)
}

/// Counts physical source lines that contain code after comments are removed.
///
/// Blank lines and comments do not contribute to architectural source-size limits.
pub(super) fn code_line_count(source: &str, hash_comments: bool) -> usize {
    code_line_flags(source, hash_comments)
        .into_iter()
        .filter(|contains_code| *contains_code)
        .count()
}

/// Counts code-bearing lines inside an inclusive one-based source range.
pub(super) fn code_line_count_between(
    source: &str,
    start_line: usize,
    end_line: usize,
    hash_comments: bool,
) -> usize {
    code_line_flags(source, hash_comments)
        .into_iter()
        .enumerate()
        .filter(|(index, contains_code)| {
            let line = index + 1;
            *contains_code && line >= start_line && line <= end_line
        })
        .count()
}

fn code_line_flags(source: &str, hash_comments: bool) -> Vec<bool> {
    let mut in_block_comment = false;
    source
        .lines()
        .map(|line| !strip_comments(line, &mut in_block_comment, hash_comments).trim().is_empty())
        .collect()
}

#[derive(Default)]
struct FunctionScanner {
    functions: Vec<FunctionRange>,
    active: Option<(usize, String)>,
    depth: usize,
    body_started: bool,
}

impl FunctionScanner {
    fn scan(source: &str, hash_comments: bool) -> Vec<FunctionRange> {
        let lines = source.lines().collect::<Vec<_>>();
        let mut scanner = Self::default();
        let mut in_block_comment = false;
        let mut in_doc_string = false;
        for (index, line) in lines.iter().enumerate() {
            scanner.scan_line(
                index,
                line,
                &mut in_block_comment,
                &mut in_doc_string,
                hash_comments,
            );
        }
        scanner.finish(lines.len());
        scanner.functions
    }

    fn scan_line(
        &mut self,
        index: usize,
        line: &str,
        in_block_comment: &mut bool,
        in_doc_string: &mut bool,
        hash_comments: bool,
    ) {
        let cleaned =
            strip_non_code_with_doc_state(line, in_block_comment, in_doc_string, hash_comments);
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

fn strip_non_code_with_doc_state(
    line: &str,
    in_block_comment: &mut bool,
    in_doc_string: &mut bool,
    hash_comments: bool,
) -> String {
    let mut output = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let characters = line.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        let next = characters.get(index + 1).copied();
        if *in_doc_string {
            if character == '"'
                && characters.get(index + 1) == Some(&'"')
                && characters.get(index + 2) == Some(&'"')
            {
                *in_doc_string = false;
                index += 3;
            } else {
                index += 1;
            }
            continue;
        }
        if !in_string
            && character == '"'
            && characters.get(index + 1) == Some(&'"')
            && characters.get(index + 2) == Some(&'"')
        {
            *in_doc_string = true;
            index += 3;
            continue;
        }
        if let Some(next_index) = consume_block_comment(&characters, index, in_block_comment) {
            index = next_index;
            continue;
        }
        if !in_string && character == '/' && next == Some('/') {
            break;
        }
        if hash_comments && !in_string && character == '#' {
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

fn strip_comments(line: &str, in_block_comment: &mut bool, hash_comments: bool) -> String {
    let mut output = String::new();
    let mut in_string = false;
    let mut in_doc_string = false;
    let mut escaped = false;
    let characters = line.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        let next = characters.get(index + 1).copied();
        if in_doc_string {
            output.push(character);
            if character == '"'
                && characters.get(index + 1) == Some(&'"')
                && characters.get(index + 2) == Some(&'"')
            {
                output.push('"');
                output.push('"');
                in_doc_string = false;
                index += 3;
            } else {
                index += 1;
            }
            continue;
        }
        if !in_string
            && character == '"'
            && characters.get(index + 1) == Some(&'"')
            && characters.get(index + 2) == Some(&'"')
        {
            output.push('"');
            output.push('"');
            output.push('"');
            in_doc_string = true;
            index += 3;
            continue;
        }
        if let Some(next_index) = consume_block_comment(&characters, index, in_block_comment) {
            index = next_index;
            continue;
        }
        if !in_string && character == '/' && next == Some('/') {
            break;
        }
        if hash_comments && !in_string && character == '#' {
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
