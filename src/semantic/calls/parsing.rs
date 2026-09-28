use crate::lexer::SourceSpan;

pub(crate) fn parse_type_name_key(key: &str, span: SourceSpan) -> Option<crate::ast::TypeName> {
    let (reference_role, key) = reference_role_prefix(key);
    if key.is_empty() || key.contains(']') && !key.contains('[') {
        return None;
    }
    let Some(open) = key.find('[') else {
        return Some(crate::ast::TypeName {
            name: key.to_owned(),
            arguments: Vec::new(),
            reference_role,
            span,
        });
    };
    if !key.ends_with(']') {
        return None;
    }
    if open == 0 || key[..open].trim().is_empty() {
        return None;
    }
    let name = key[..open].to_owned();
    let inner = &key[open + 1..key.len() - 1];
    let arguments = split_type_arguments(inner)?
        .into_iter()
        .map(|argument| parse_type_name_key(argument, span))
        .collect::<Option<Vec<_>>>()?;
    Some(crate::ast::TypeName { name, arguments, reference_role, span })
}

fn reference_role_prefix(key: &str) -> (Option<crate::ast::Role>, &str) {
    for (prefix, role) in [("abs ", crate::ast::Role::Abs), ("ins ", crate::ast::Role::Ins)] {
        if let Some(name) = key.strip_prefix(prefix) {
            return (Some(role), name);
        }
    }
    (None, key)
}

fn split_type_arguments(input: &str) -> Option<Vec<&str>> {
    let mut depth = 0;
    let mut start = 0;
    let mut parts = Vec::new();
    for (index, character) in input.char_indices() {
        match character {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            ']' => return None,
            ',' if depth == 0 => {
                let part = input[start..index].trim();
                if part.is_empty() {
                    return None;
                }
                parts.push(part);
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    let part = input[start..].trim();
    if part.is_empty() {
        return None;
    }
    parts.push(part);
    Some(parts)
}
