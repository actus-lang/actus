use crate::semantic::SemanticErrorKind;

use super::pack_codes;

pub(super) fn extended_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    enum_semantic_message(kind)
        .or_else(|| guard_semantic_message(kind))
        .or_else(|| struct_semantic_message(kind))
        .or_else(|| pack_codes::message(kind))
        .or_else(|| arena_semantic_message(kind))
        .or_else(|| type_semantic_message(kind))
}

fn arena_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::InvalidArenaCapacity { capacity } => {
            format!("arena capacity `{capacity}` must be a positive compile-time byte count")
        }
        SemanticErrorKind::ArenaReferenceEscape { name } => {
            format!("arena-derived reference `{name}` cannot escape its arena scope")
        }
        SemanticErrorKind::CrossArenaReference { name } => {
            format!("value `{name}` belongs to a different arena provenance")
        }
        SemanticErrorKind::ArenaReferenceLive { arena, reference } => {
            format!("cannot move or drop arena `{arena}` while reference `{reference}` is live")
        }
        _ => return None,
    };
    Some(message)
}

fn enum_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    enum_declaration_message(kind).or_else(|| enum_pattern_message(kind))
}

fn enum_declaration_message(kind: &SemanticErrorKind) -> Option<String> {
    enum_type_message(kind).or_else(|| enum_variant_message(kind))
}

fn enum_type_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::DuplicateEnumName { name } => {
            format!("duplicate enum declaration `{name}`")
        }
        SemanticErrorKind::EmptyEnum { name } => {
            format!("enum `{name}` must declare at least one variant")
        }
        SemanticErrorKind::RecursiveType { name } => {
            format!("recursive type `{name}` requires indirection")
        }
        _ => return None,
    };
    Some(message)
}

fn enum_variant_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::UnknownEnumVariant { enum_name, variant } => {
            format!("unknown variant `{variant}` for enum `{enum_name}`")
        }
        SemanticErrorKind::EnumVariantArgumentCount { enum_name, variant, expected, found } => {
            format!(
                "wrong argument count for `{enum_name}.{variant}`: expected {expected}, found {found}"
            )
        }
        SemanticErrorKind::EnumVariantArgumentName { enum_name, variant, name } => {
            format!("unknown field `{name}` for variant `{enum_name}.{variant}`")
        }
        SemanticErrorKind::EnumVariantArgumentTypeMismatch {
            variant,
            parameter,
            expected,
            found,
        } => {
            format!(
                "type mismatch for variant `{variant}` field `{parameter}`: expected `{expected}`, found `{found}`"
            )
        }
        _ => return None,
    };
    Some(message)
}

fn enum_pattern_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::NonExhaustiveMatch { subject, missing } => {
            format!("non-exhaustive match on `{subject}`; missing: {}", missing.join(", "))
        }
        SemanticErrorKind::UnreachablePattern { pattern } => {
            format!("unreachable pattern `{pattern}`")
        }
        SemanticErrorKind::DuplicatePattern { pattern } => format!("duplicate pattern `{pattern}`"),
        SemanticErrorKind::PatternTypeMismatch { expected, found } => {
            format!("pattern type mismatch: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::PatternBindingTypeMismatch { binding, expected, found } => {
            format!("pattern binding `{binding}` has type `{found}`, expected `{expected}`")
        }
        SemanticErrorKind::InvalidCaseRole { mode, subject } => {
            format!("cannot use `{mode}` case deconstruction on `{subject}`")
        }
        SemanticErrorKind::InvalidMutation { name } => {
            format!("cannot mutate read-only binding `{name}`")
        }
        _ => return None,
    };
    Some(message)
}

fn guard_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::InvalidGuardAccess { name } => {
            format!("case guard `{name}` requires read-only access")
        }
        SemanticErrorKind::GuardTypeMismatch { found } => {
            format!("case guard must return `Bool`, found `{found}`")
        }
        SemanticErrorKind::BranchStateMismatch { name, expected, found } => {
            format!(
                "case branch state mismatch for `{name}`: expected `{expected}`, found `{found}`"
            )
        }
        _ => return None,
    };
    Some(message)
}

fn struct_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    struct_declaration_message(kind).or_else(|| struct_field_message(kind))
}

fn struct_declaration_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::DuplicateStructName { name } => {
            format!("duplicate struct declaration `{name}`")
        }
        SemanticErrorKind::DuplicatePackName { name } => {
            format!("duplicate pack declaration `{name}`")
        }
        _ => return None,
    };
    Some(message)
}

fn struct_field_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::DuplicateStructField { struct_name, field } => {
            format!("duplicate field `{field}` in struct `{struct_name}`")
        }
        SemanticErrorKind::UnknownStructField { struct_name, field } => {
            format!("unknown field `{field}` on struct `{struct_name}`")
        }
        SemanticErrorKind::MissingStructField { struct_name, field } => {
            format!("missing field `{field}` in `{struct_name}` initializer")
        }
        SemanticErrorKind::StructFieldTypeMismatch { struct_name, field, expected, found } => {
            format!(
                "type mismatch for field `{field}` in `{struct_name}`: expected `{expected}`, found `{found}`"
            )
        }
        SemanticErrorKind::InvalidFieldAssignmentTarget { field } => {
            format!("field `{field}` can only be assigned through an erg owner")
        }
        SemanticErrorKind::FieldBorrowConflict { owner, field, .. } => {
            format!("cannot mutate or move field `{field}` because owner `{owner}` is frozen")
        }
        _ => return None,
    };
    Some(message)
}

fn type_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    type_contract_message(kind)
        .or_else(|| type_role_message(kind))
        .or_else(|| type_result_message(kind))
}

fn type_role_message(kind: &SemanticErrorKind) -> Option<String> {
    type_role_declaration_message(kind).or_else(|| type_signature_message(kind))
}

fn type_role_declaration_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::DuplicateRoleName { name } => {
            format!("duplicate role declaration `{name}`")
        }
        SemanticErrorKind::UnknownRole { name } => format!("unknown role `{name}`"),
        SemanticErrorKind::DuplicateRoleMethod { role, method } => {
            format!("duplicate method `{method}` in role `{role}`")
        }
        SemanticErrorKind::MissingRoleMethod { role, method } => {
            format!("performance for role `{role}` is missing method `{method}`")
        }
        SemanticErrorKind::RoleMethodMismatch { role, method } => {
            format!("method `{method}` does not satisfy role `{role}`")
        }
        SemanticErrorKind::InvalidRoleReceiver { role, method } => {
            format!("method `{method}` in role `{role}` must declare an explicit `self` receiver")
        }
        _ => return None,
    };
    Some(message)
}

fn type_signature_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::DuplicateVerbName { name } => {
            format!("duplicate verb declaration `{name}`")
        }
        SemanticErrorKind::TypeMismatch { callee, parameter, expected, found } => format!(
            "type mismatch for `{parameter}` in `{callee}`: expected `{expected}`, found `{found}`"
        ),
        SemanticErrorKind::UnresolvedResultConstructor { constructor } => {
            format!("cannot infer `{constructor}` here; use `Result[T, E].{constructor}(...)`")
        }
        SemanticErrorKind::ReturnTypeMismatch { expected, found } => {
            format!("return type mismatch: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::BindingTypeMismatch { binding, expected, found } => {
            format!("type mismatch for `{binding}`: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::UnknownMethod { method } => format!("unknown method `{method}`"),
        SemanticErrorKind::InvalidReceiver { method } => {
            format!("invalid receiver for method `{method}`")
        }
        _ => return None,
    };
    Some(message)
}

fn type_result_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::ReceiverTypeMismatch { method, expected, found } => {
            format!("receiver type mismatch for `{method}`: expected `{expected}`, found `{found}")
        }
        SemanticErrorKind::MissingReturnValue => {
            "verb must return a value on every path".to_owned()
        }
        _ => return None,
    };
    Some(message)
}

fn type_contract_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::UnknownType { name } => format!("unknown type `{name}`"),
        SemanticErrorKind::MalformedTypeName { name } => format!("malformed type name `{name}`"),
        SemanticErrorKind::UnknownTypeParameter { name } => {
            format!("undeclared generic type parameter `{name}`")
        }
        SemanticErrorKind::GenericArityMismatch { name, expected, found } => {
            format!(
                "wrong number of type arguments for `{name}`: expected {expected}, found {found}"
            )
        }
        SemanticErrorKind::GenericConstraintMismatch { parameter, constraint, argument } => {
            format!("generic parameter `{parameter}` requires `{constraint}`, found `{argument}`")
        }
        SemanticErrorKind::TypeMismatch { callee, parameter, expected, found } => format!(
            "type mismatch for `{parameter}` in `{callee}`: expected `{expected}`, found `{found}`"
        ),
        SemanticErrorKind::ReturnTypeMismatch { expected, found } => {
            format!("return type mismatch: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::BindingTypeMismatch { binding, expected, found } => {
            format!("type mismatch for `{binding}`: expected `{expected}`, found `{found}`")
        }
        _ => return None,
    };
    Some(message)
}
