use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FunctionRange {
    pub(super) name: String,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
}

pub(super) fn find_functions(source: &str) -> Vec<FunctionRange> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut functions = Vec::new();
    let mut active = None;
    let mut depth = 0usize;
    let mut body_started = false;
    let mut in_block_comment = false;
    for (index, line) in lines.iter().enumerate() {
        let cleaned = strip_non_code(line, &mut in_block_comment);
        if active.is_none()
            && let Some(name) = declaration_name(&cleaned)
        {
            active = Some((index + 1, name));
            depth = 0;
            body_started = false;
        }
        let Some((start_line, name)) = &active else { continue };
        let (opens, closes) = brace_counts(&cleaned);
        if opens > 0 {
            body_started = true;
        }
        depth = depth.saturating_add(opens).saturating_sub(closes);
        if body_started && depth == 0 {
            functions.push(FunctionRange {
                name: name.clone(),
                start_line: *start_line,
                end_line: index + 1,
            });
            active = None;
        } else if !body_started && cleaned.contains(';') {
            active = None;
        }
    }
    if let Some((start_line, name)) = active {
        functions.push(FunctionRange { name, start_line, end_line: lines.len() });
    }
    functions
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
    let name = name.split(['(', '<', ':']).next().unwrap_or(name);
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
        if *in_block_comment {
            if character == '*' && next == Some('/') {
                *in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
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
        if character == '"' && !escaped {
            in_string = !in_string;
        } else if !in_string {
            output.push(character);
        }
        escaped = character == '\\' && !escaped;
        if character != '\\' {
            escaped = false;
        }
        index += 1;
    }
    output
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
