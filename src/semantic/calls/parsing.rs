use crate::lexer::SourceSpan;

pub(crate) fn parse_type_name_key(key: &str, span: SourceSpan) -> Option<crate::ast::TypeName> {
    let Some(open) = key.find('[') else {
        return Some(crate::ast::TypeName { name: key.to_owned(), arguments: Vec::new(), span });
    };
    if !key.ends_with(']') {
        return None;
    }
    let name = key[..open].to_owned();
    let inner = &key[open + 1..key.len() - 1];
    let arguments = split_type_arguments(inner)
        .into_iter()
        .map(|argument| parse_type_name_key(argument, span))
        .collect::<Option<Vec<_>>>()?;
    Some(crate::ast::TypeName { name, arguments, span })
}

fn split_type_arguments(input: &str) -> Vec<&str> {
    let mut depth = 0;
    let mut start = 0;
    let mut parts = Vec::new();
    for (index, character) in input.char_indices() {
        match character {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < input.len() {
        parts.push(input[start..].trim());
    }
    parts
}
