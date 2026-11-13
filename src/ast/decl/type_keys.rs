use crate::lexer::SourceSpan;

use super::{Role, TypeName};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeNameParseMode {
    Strict,
    Permissive,
    Plain,
}

pub(crate) fn parse_type_name_key(key: &str, span: SourceSpan) -> Option<TypeName> {
    parse_type_name_with_mode(key, span, TypeNameParseMode::Strict)
}

pub(crate) fn parse_type_name_with_mode(
    input: &str,
    span: SourceSpan,
    mode: TypeNameParseMode,
) -> Option<TypeName> {
    let (reference_role, key) = match mode {
        TypeNameParseMode::Plain => (None, input),
        TypeNameParseMode::Strict | TypeNameParseMode::Permissive => reference_role_prefix(input),
    };
    if matches!(mode, TypeNameParseMode::Strict)
        && (key.is_empty() || key.contains(']') && !key.contains('['))
    {
        return None;
    }
    let Some(open) = key.find('[') else {
        return Some(TypeName {
            name: key.to_owned(),
            arguments: Vec::new(),
            reference_role,
            span,
        });
    };
    if matches!(mode, TypeNameParseMode::Strict)
        && (!key.ends_with(']') || open == 0 || key[..open].trim().is_empty())
    {
        return None;
    }
    if !key.ends_with(']') {
        return None;
    }
    let inner = &key[open + 1..key.len() - 1];
    let parts = match mode {
        TypeNameParseMode::Plain => inner.split(',').map(str::trim).collect(),
        TypeNameParseMode::Strict => split_nested_arguments(inner)?,
        TypeNameParseMode::Permissive => split_nested_arguments_permissive(inner),
    };
    let arguments = parts
        .into_iter()
        .map(|argument| parse_type_name_with_mode(argument, span, mode))
        .collect::<Option<Vec<_>>>()?;
    Some(TypeName { name: key[..open].to_owned(), arguments, reference_role, span })
}

fn reference_role_prefix(key: &str) -> (Option<Role>, &str) {
    for (prefix, role) in [("abs ", Role::Abs), ("ins ", Role::Ins)] {
        if let Some(name) = key.strip_prefix(prefix) {
            return (Some(role), name);
        }
    }
    (None, key)
}

fn split_nested_arguments(input: &str) -> Option<Vec<&str>> {
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

fn split_nested_arguments_permissive(input: &str) -> Vec<&str> {
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

#[cfg(test)]
mod tests {
    use super::{TypeNameParseMode, parse_type_name_key, parse_type_name_with_mode};
    use crate::ast::Role;
    use crate::lexer::SourceSpan;

    fn span() -> SourceSpan {
        SourceSpan::new(0, 1)
    }

    #[test]
    fn strict_mode_preserves_roles_and_nested_arguments() {
        let type_name = parse_type_name_key("abs Result[Int, Array[u8, 4]]", span()).unwrap();
        assert_eq!(type_name.reference_role, Some(Role::Abs));
        assert_eq!(type_name.arguments[1].arguments.len(), 2);
    }

    #[test]
    fn strict_mode_rejects_malformed_keys() {
        for key in ["", "Result[", "Result[]", "Result[Int,]", "]Int"] {
            assert!(parse_type_name_key(key, span()).is_none(), "accepted {key:?}");
        }
    }

    #[test]
    fn permissive_mode_matches_enum_constructor_parsing() {
        let type_name = parse_type_name_with_mode(
            "Result[Int, Array[u8, 4]]",
            span(),
            TypeNameParseMode::Permissive,
        )
        .unwrap();
        assert_eq!(type_name.arguments[1].arguments.len(), 2);
        assert_eq!(
            parse_type_name_with_mode("Result[]", span(), TypeNameParseMode::Permissive)
                .unwrap()
                .arguments
                .len(),
            0
        );
    }

    #[test]
    fn plain_mode_keeps_plain_comma_splitting_and_no_role() {
        let type_name =
            parse_type_name_with_mode("Result[Int,Array]", span(), TypeNameParseMode::Plain)
                .unwrap();
        assert_eq!(type_name.reference_role, None);
        assert_eq!(type_name.arguments.len(), 2);
        assert!(
            parse_type_name_with_mode("Result[Int,Array[u8,4]]", span(), TypeNameParseMode::Plain,)
                .is_none()
        );
    }
}
