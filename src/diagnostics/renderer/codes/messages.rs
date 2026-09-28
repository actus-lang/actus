use crate::semantic::SemanticErrorKind;

use super::extended;

pub(crate) fn semantic_message(kind: &SemanticErrorKind) -> String {
    if let SemanticErrorKind::NumericLiteralOutOfRange { ty, literal } = kind {
        return format!("literal `{literal}` is outside the range of `{ty}`");
    }
    if let Some(message) = ownership_semantic_message(kind) {
        return message;
    }
    if let Some(message) = extended::extended_semantic_message(kind) {
        return message;
    }
    unreachable!("extended semantic message was not rendered")
}

fn ownership_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    ownership_core_message(kind).or_else(|| ownership_call_message(kind))
}

fn ownership_core_message(kind: &SemanticErrorKind) -> Option<String> {
    Some(match kind {
        SemanticErrorKind::DuplicateBinding { name } => format!("duplicate binding `{name}`"),
        SemanticErrorKind::ShadowedBinding { name } => format!("shadowed binding `{name}`"),
        SemanticErrorKind::UndeclaredIdentifier { name } => {
            format!("undeclared identifier `{name}`")
        }
        SemanticErrorKind::InvalidBorrowTarget { name } => {
            format!("invalid borrow target `{name}`")
        }
        SemanticErrorKind::UseAfterMove { name } => format!("use of moved binding `{name}`"),
        SemanticErrorKind::UseAfterDrop { name } => format!("use of dropped binding `{name}`"),
        SemanticErrorKind::DoubleDrop { name } => format!("binding `{name}` was dropped twice"),
        SemanticErrorKind::DropBorrow { name } => format!("cannot drop borrow `{name}` directly"),
        SemanticErrorKind::DropFrozen { name, .. } => {
            format!("cannot drop frozen binding `{name}`")
        }
        SemanticErrorKind::InvalidDatArgument { name } => format!("invalid dat argument `{name}`"),
        SemanticErrorKind::MoveFrozen { name, .. } => {
            format!("cannot move frozen binding `{name}`")
        }
        _ => return None,
    })
}

fn ownership_call_message(kind: &SemanticErrorKind) -> Option<String> {
    Some(match kind {
        SemanticErrorKind::UnknownParameter { callee, name } => {
            format!("unknown parameter `{name}` in call to `{callee}`")
        }
        SemanticErrorKind::UnknownVerb { name } => format!("unknown verb `{name}`"),
        SemanticErrorKind::DuplicateArgument { name } => format!("duplicate argument `{name}`"),
        SemanticErrorKind::MixedArgumentModes { callee } => {
            format!("cannot mix named and positional arguments in `{callee}`")
        }
        SemanticErrorKind::WrongArgumentCount { callee } => {
            format!("wrong argument count in `{callee}`")
        }
        SemanticErrorKind::InvalidArgumentRole { callee, parameter } => {
            format!("argument does not satisfy role of `{parameter}` in `{callee}`")
        }
        SemanticErrorKind::InvalidAbsReturnOrigin { reason } => {
            format!("invalid abs return origin: {reason}")
        }
        SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter } => {
            format!("invalid `{parameter}` argument in intrinsic `{callee}`")
        }
        SemanticErrorKind::AmbiguousPositionalCall { callee } => {
            format!("positional call to `{callee}` is ambiguous")
        }
        SemanticErrorKind::BorrowedReturn { name } => {
            format!("borrow `{name}` cannot escape its scope")
        }
        SemanticErrorKind::EscapingLoan { name } => {
            format!("exclusive loan `{name}` cannot escape its call scope")
        }
        SemanticErrorKind::InvalidOwnerInitializer { name } => {
            format!("invalid owner initializer `{name}`")
        }
        SemanticErrorKind::LoopControlOutsideLoop { keyword } => {
            format!("`{keyword}` is only valid inside a loop")
        }
        SemanticErrorKind::ReservedIntrinsicName { name } => {
            format!("`{name}` is reserved for a built-in intrinsic")
        }
        SemanticErrorKind::SuspendedAccess { name, loan_id } => {
            format!("binding `{name}` is suspended by exclusive loan #{loan_id}")
        }
        SemanticErrorKind::ExclusiveLoanAlias { name } => {
            format!("exclusive loan aliases resource `{name}` more than once")
        }
        _ => return None,
    })
}
