use crate::lexer::SourceSpan;

use super::{DeclarationIdentity, DeclarationPath, Role, TypeName};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeNameParseMode {
    Strict,
    Permissive,
}

pub(crate) fn parse_type_name_key(key: &str, span: SourceSpan) -> Option<TypeName> {
    parse_type_name_with_mode(key, span, TypeNameParseMode::Strict)
}

pub fn encode_type_key(type_name: &TypeName) -> String {
    let mut encoded = String::from("T");
    encoded.push(match type_name.reference_role {
        None => '-',
        Some(Role::Erg) => 'e',
        Some(Role::Abs) => 'a',
        Some(Role::Dat) => 'd',
        Some(Role::Ins) => 'i',
    });
    match &type_name.resolved {
        Some(identity) => encode_identity(identity, &mut encoded),
        None => {
            encoded.push('U');
        }
    }
    encode_string(&type_name.name, &mut encoded);
    encoded.push('A');
    encoded.push_str(&type_name.arguments.len().to_string());
    encoded.push(':');
    for argument in &type_name.arguments {
        encoded.push_str(&encode_type_key(argument));
    }
    encoded
}

pub fn decode_type_key(key: &str, span: SourceSpan) -> Option<TypeName> {
    let mut cursor = 0;
    let type_name = decode_type_key_at(key, &mut cursor, span)?;
    (cursor == key.len()).then_some(type_name)
}

fn encode_identity(identity: &DeclarationIdentity, output: &mut String) {
    match identity {
        DeclarationIdentity::Declaration(path) => {
            output.push('D');
            encode_string(path.package(), output);
            encode_string(path.module_path(), output);
            encode_string(path.name(), output);
        }
        DeclarationIdentity::Builtin { name } => {
            output.push('B');
            encode_string(name, output);
        }
        DeclarationIdentity::GenericParameter { owner, name } => {
            output.push('G');
            encode_string(owner, output);
            encode_string(name, output);
        }
        DeclarationIdentity::Const { value } => {
            output.push('C');
            encode_string(value, output);
        }
        DeclarationIdentity::Unknown { spelling } => {
            output.push('X');
            encode_string(spelling, output);
        }
    }
}

fn encode_string(value: &str, output: &mut String) {
    output.push_str(&value.len().to_string());
    output.push(':');
    output.push_str(value);
}

fn decode_type_key_at(key: &str, cursor: &mut usize, span: SourceSpan) -> Option<TypeName> {
    if key.as_bytes().get(*cursor).copied()? != b'T' {
        return None;
    }
    *cursor += 1;
    let role = match key.as_bytes().get(*cursor).copied()? {
        b'-' => None,
        b'e' => Some(Role::Erg),
        b'a' => Some(Role::Abs),
        b'd' => Some(Role::Dat),
        b'i' => Some(Role::Ins),
        _ => return None,
    };
    *cursor += 1;
    let resolved = match key.as_bytes().get(*cursor).copied()? {
        b'U' => {
            *cursor += 1;
            None
        }
        b'D' | b'B' | b'G' | b'C' | b'X' => Some(decode_identity(key, cursor)?),
        _ => return None,
    };
    let spelling = decode_string(key, cursor)?;
    if key.as_bytes().get(*cursor)? != &b'A' {
        return None;
    }
    *cursor += 1;
    let count = decode_decimal(key, cursor)?;
    let arguments =
        (0..count).map(|_| decode_type_key_at(key, cursor, span)).collect::<Option<Vec<_>>>()?;
    Some(match resolved {
        Some(identity) => TypeName::resolved(identity, spelling, arguments, role, span),
        None => TypeName::unresolved(spelling, arguments, role, span),
    })
}

fn decode_identity(key: &str, cursor: &mut usize) -> Option<DeclarationIdentity> {
    let tag = key.as_bytes().get(*cursor).copied()? as char;
    *cursor += 1;
    match tag {
        'D' => Some(DeclarationIdentity::Declaration(DeclarationPath::decoded(
            decode_string(key, cursor)?,
            decode_string(key, cursor)?,
            decode_string(key, cursor)?,
        ))),
        'B' => Some(DeclarationIdentity::Builtin { name: decode_string(key, cursor)? }),
        'G' => Some(DeclarationIdentity::GenericParameter {
            owner: decode_string(key, cursor)?,
            name: decode_string(key, cursor)?,
        }),
        'C' => Some(DeclarationIdentity::Const { value: decode_string(key, cursor)? }),
        'X' => Some(DeclarationIdentity::Unknown { spelling: decode_string(key, cursor)? }),
        _ => None,
    }
}

fn decode_string(key: &str, cursor: &mut usize) -> Option<String> {
    let length = decode_decimal(key, cursor)?;
    let start = *cursor;
    let end = start.checked_add(length)?;
    let value = key.get(start..end)?.to_owned();
    *cursor = end;
    Some(value)
}

fn decode_decimal(key: &str, cursor: &mut usize) -> Option<usize> {
    let rest = key.get(*cursor..)?;
    let separator = rest.find(':')?;
    let value = rest[..separator].parse().ok()?;
    *cursor += separator + 1;
    Some(value)
}

pub(crate) fn parse_type_name_with_mode(
    input: &str,
    span: SourceSpan,
    mode: TypeNameParseMode,
) -> Option<TypeName> {
    let (reference_role, key) = match mode {
        TypeNameParseMode::Strict | TypeNameParseMode::Permissive => reference_role_prefix(input),
    };
    if key.chars().next().is_some_and(|character| character.is_ascii_digit()) {
        return Some(TypeName::constant(key, key, span));
    }
    if matches!(mode, TypeNameParseMode::Strict)
        && (key.is_empty() || key.contains(']') && !key.contains('['))
    {
        return None;
    }
    let Some(open) = key.find('[') else {
        return Some(TypeName::unresolved(key, Vec::new(), reference_role, span));
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
        TypeNameParseMode::Strict => split_nested_arguments(inner)?,
        TypeNameParseMode::Permissive => split_nested_arguments_permissive(inner),
    };
    let arguments = parts
        .into_iter()
        .map(|argument| parse_type_name_with_mode(argument, span, mode))
        .collect::<Option<Vec<_>>>()?;
    Some(TypeName::unresolved(key[..open].to_owned(), arguments, reference_role, span))
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
    use crate::ast::{DeclarationIdentity, Role, TypeName, decode_type_key, encode_type_key};
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
    fn type_keys_round_trip_all_identity_variants_and_arguments() {
        let identities = [
            DeclarationIdentity::Declaration(super::DeclarationPath::decoded(
                "app_pkg",
                "interface::config",
                "ParseError",
            )),
            DeclarationIdentity::Builtin { name: "Result".to_owned() },
            DeclarationIdentity::GenericParameter {
                owner: "BoxOwner".to_owned(),
                name: "T".to_owned(),
            },
            DeclarationIdentity::Const { value: "64".to_owned() },
        ];
        for identity in identities {
            let type_name = TypeName::resolved(
                identity,
                "user spelling::T",
                vec![TypeName::resolved(
                    DeclarationIdentity::Builtin { name: "Int".to_owned() },
                    "Int",
                    Vec::new(),
                    Some(Role::Abs),
                    span(),
                )],
                Some(Role::Ins),
                span(),
            );
            let decoded = decode_type_key(&encode_type_key(&type_name), span()).unwrap();
            assert_eq!(decoded.resolved, type_name.resolved);
            assert_eq!(decoded.arguments, type_name.arguments);
            assert_eq!(decoded.reference_role, type_name.reference_role);
            assert_eq!(decoded.name, type_name.name);
        }
    }
}
