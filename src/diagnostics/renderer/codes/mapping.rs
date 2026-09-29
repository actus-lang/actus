use crate::lexer::{LexError, LexErrorKind};
use crate::parser::ParseErrorCode;
use crate::semantic::SemanticErrorKind;

use super::pack_codes;

pub(crate) fn lex_code(error: &LexError) -> u8 {
    match error.kind {
        LexErrorKind::UnexpectedCharacter(_) => 1,
        LexErrorKind::UnterminatedString => 2,
        LexErrorKind::UnterminatedDocString => 3,
        LexErrorKind::InvalidIntegerType(_) => 4,
        LexErrorKind::InvalidHexLiteral(_) => 5,
    }
}

pub(crate) fn parse_code(code: ParseErrorCode) -> &'static str {
    match code {
        ParseErrorCode::UnexpectedToken => "E0003",
        ParseErrorCode::UnexpectedEndOfInput => "E0004",
        ParseErrorCode::DuplicateName => "E0005",
        ParseErrorCode::UnknownKeyword => "E0009",
        ParseErrorCode::UnknownMetadata => "E0006",
        ParseErrorCode::UnsupportedTargetPlatform => "E0007",
        ParseErrorCode::ConflictingTargetPlatforms => "E0010",
        ParseErrorCode::MetadataTargetNotAllowed => "E0008",
    }
}

pub(crate) fn semantic_code(kind: &SemanticErrorKind) -> &'static str {
    if let Some(code) = pack_codes::code(kind) {
        return code;
    }
    if let Some(code) = role_semantic_code(kind) {
        return code;
    }
    if let Some(code) = type_semantic_code(kind) {
        return code;
    }
    if let Some(code) = ownership_semantic_code(kind) {
        return code;
    }
    declaration_semantic_code(kind)
        .or_else(|| expression_semantic_code(kind))
        .unwrap_or_else(|| unreachable!("extended semantic diagnostic code handled above"))
}

fn declaration_semantic_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::DuplicateVerbName { .. } => "E1024",
        SemanticErrorKind::DuplicateStructName { .. } => "E1029",
        SemanticErrorKind::DuplicateEnumName { .. } => "E1039",
        SemanticErrorKind::DuplicatePackName { .. } => "E1081",
        SemanticErrorKind::EmptyEnum { .. } => "E1810",
        SemanticErrorKind::DuplicateStructField { .. } => "E1030",
        SemanticErrorKind::UnknownStructField { .. } => "E1031",
        SemanticErrorKind::MissingStructField { .. } => "E1032",
        SemanticErrorKind::StructFieldTypeMismatch { .. } => "E1033",
        _ => return None,
    })
}

fn expression_semantic_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    expression_value_code(kind).or_else(|| expression_pattern_code(kind))
}

fn expression_value_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::TypeMismatch { .. } => "E1025",
        SemanticErrorKind::UnknownVerb { .. } => "E1069",
        SemanticErrorKind::UnresolvedResultConstructor { .. } => "E1067",
        SemanticErrorKind::NumericLiteralOutOfRange { .. } => "E1068",
        SemanticErrorKind::InvalidArenaCapacity { .. } => "E1076",
        SemanticErrorKind::ArenaReferenceEscape { .. } => "E1077",
        SemanticErrorKind::CrossArenaReference { .. } => "E1078",
        SemanticErrorKind::ArenaReferenceLive { .. } => "E1079",
        SemanticErrorKind::ReturnTypeMismatch { .. } => "E1026",
        SemanticErrorKind::BindingTypeMismatch { .. } => "E1027",
        SemanticErrorKind::MissingReturnValue => "E1028",
        SemanticErrorKind::InvalidFieldAssignmentTarget { .. } => "E1034",
        SemanticErrorKind::FieldBorrowConflict { .. } => "E1035",
        _ => return None,
    })
}

fn expression_pattern_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::UnknownMethod { .. } => "E1036",
        SemanticErrorKind::InvalidReceiver { .. } => "E1037",
        SemanticErrorKind::ReceiverTypeMismatch { .. } => "E1038",
        SemanticErrorKind::UnknownEnumVariant { .. } => "E1040",
        SemanticErrorKind::EnumVariantArgumentCount { .. } => "E1041",
        SemanticErrorKind::EnumVariantArgumentName { .. } => "E1042",
        SemanticErrorKind::EnumVariantArgumentTypeMismatch { .. } => "E1043",
        SemanticErrorKind::RecursiveType { .. } => "E1044",
        SemanticErrorKind::NonExhaustiveMatch { .. } => "E1045",
        SemanticErrorKind::UnreachablePattern { .. } => "E1046",
        SemanticErrorKind::DuplicatePattern { .. } => "E1047",
        SemanticErrorKind::PatternTypeMismatch { .. } => "E1048",
        SemanticErrorKind::PatternBindingTypeMismatch { .. } => "E1049",
        _ => return None,
    })
}

fn ownership_semantic_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    ownership_binding_code(kind).or_else(|| ownership_call_code(kind))
}

fn ownership_binding_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::DuplicateBinding { .. } => "E1001",
        SemanticErrorKind::ShadowedBinding { .. } => "E1002",
        SemanticErrorKind::UndeclaredIdentifier { .. } => "E1003",
        SemanticErrorKind::InvalidBorrowTarget { .. } => "E1004",
        SemanticErrorKind::UseAfterMove { .. } => "E1005",
        SemanticErrorKind::UseAfterDrop { .. } => "E1006",
        SemanticErrorKind::DoubleDrop { .. } => "E1007",
        SemanticErrorKind::DropBorrow { .. } => "E1008",
        SemanticErrorKind::DropFrozen { .. } => "E1009",
        SemanticErrorKind::InvalidDatArgument { .. } => "E1010",
        SemanticErrorKind::MoveFrozen { .. } => "E1011",
        SemanticErrorKind::UnknownParameter { .. } => "E1012",
        SemanticErrorKind::DuplicateArgument { .. } => "E1013",
        SemanticErrorKind::MixedArgumentModes { .. } => "E1014",
        SemanticErrorKind::WrongArgumentCount { .. } => "E1015",
        SemanticErrorKind::InvalidArgumentRole { .. } => "E1016",
        _ => return None,
    })
}

fn ownership_call_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::InvalidAbsReturnOrigin { .. } => "E1066",
        SemanticErrorKind::InvalidIntrinsicArgument { .. } => "E1021",
        SemanticErrorKind::AmbiguousPositionalCall { .. } => "E1017",
        SemanticErrorKind::BorrowedReturn { .. } => "E1018",
        SemanticErrorKind::InvalidOwnerInitializer { .. } => "E1019",
        SemanticErrorKind::LoopControlOutsideLoop { .. } => "E1020",
        SemanticErrorKind::ReservedIntrinsicName { .. } => "E1022",
        SemanticErrorKind::SuspendedAccess { .. } => "E1064",
        SemanticErrorKind::ExclusiveLoanAlias { .. } => "E1065",
        SemanticErrorKind::EscapingLoan { .. } => "E1082",
        _ => return None,
    })
}

fn type_semantic_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::UnknownType { .. } => "E1023",
        SemanticErrorKind::MalformedTypeName { .. } => "E1080",
        SemanticErrorKind::UnknownTypeParameter { .. } => "E1052",
        SemanticErrorKind::GenericArityMismatch { .. } => "E1053",
        SemanticErrorKind::GenericConstraintMismatch { .. } => "E1054",
        SemanticErrorKind::InvalidMutation { .. } => "E1051",
        SemanticErrorKind::InvalidArrayCapacity { .. } => "E1083",
        SemanticErrorKind::InvalidIndexType { .. } => "E1084",
        SemanticErrorKind::IndexOutOfBounds { .. } => "E1085",
        SemanticErrorKind::NonIndexableTarget { .. } => "E1086",
        SemanticErrorKind::IndexedElementTypeMismatch { .. } => "E1087",
        SemanticErrorKind::UnresolvedResultConstructor { .. } => "E1067",
        SemanticErrorKind::InvalidCaseRole { .. } => "E1050",
        SemanticErrorKind::InvalidGuardAccess { .. } => "E1062",
        SemanticErrorKind::GuardTypeMismatch { .. } => "E1063",
        SemanticErrorKind::BranchStateMismatch { .. } => "E1061",
        _ => return None,
    })
}

fn role_semantic_code(kind: &SemanticErrorKind) -> Option<&'static str> {
    Some(match kind {
        SemanticErrorKind::DuplicateRoleName { .. } => "E1055",
        SemanticErrorKind::UnknownRole { .. } => "E1056",
        SemanticErrorKind::DuplicateRoleMethod { .. } => "E1057",
        SemanticErrorKind::MissingRoleMethod { .. } => "E1058",
        SemanticErrorKind::RoleMethodMismatch { .. } => "E1059",
        SemanticErrorKind::InvalidRoleReceiver { .. } => "E1060",
        _ => return None,
    })
}
