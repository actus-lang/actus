use crate::diagnostics::DiagnosticCode;
use crate::lexer::{LexError, LexErrorKind};
use crate::parser::ParseErrorCode;
use crate::semantic::SemanticErrorKind;

use super::packing;

pub(crate) fn lex_code(error: &LexError) -> DiagnosticCode {
    match error.kind {
        LexErrorKind::UnexpectedCharacter(_) => DiagnosticCode::LEX_UNEXPECTED_CHARACTER,
        LexErrorKind::UnterminatedString => DiagnosticCode::LEX_UNTERMINATED_STRING,
        LexErrorKind::UnterminatedDocString => DiagnosticCode::LEX_UNTERMINATED_DOC_STRING,
        LexErrorKind::InvalidIntegerType(_) => DiagnosticCode::LEX_INVALID_INTEGER_TYPE,
        LexErrorKind::InvalidHexLiteral(_) => DiagnosticCode::LEX_INVALID_HEX_LITERAL,
        LexErrorKind::InvalidFloatType(_) => DiagnosticCode::LEX_INVALID_FLOAT_TYPE,
    }
}

pub(crate) fn parse_code(code: ParseErrorCode) -> DiagnosticCode {
    match code {
        ParseErrorCode::UnexpectedToken => DiagnosticCode::PARSE_UNEXPECTED_TOKEN,
        ParseErrorCode::UnexpectedEndOfInput => DiagnosticCode::PARSE_UNEXPECTED_END_OF_INPUT,
        ParseErrorCode::DuplicateName => DiagnosticCode::PARSE_DUPLICATE_NAME,
        ParseErrorCode::UnknownKeyword => DiagnosticCode::PARSE_UNKNOWN_KEYWORD,
        ParseErrorCode::UnknownMetadata => DiagnosticCode::PARSE_UNKNOWN_METADATA,
        ParseErrorCode::UnsupportedTargetPlatform => {
            DiagnosticCode::PARSE_UNSUPPORTED_TARGET_PLATFORM
        }
        ParseErrorCode::ConflictingTargetPlatforms => {
            DiagnosticCode::PARSE_CONFLICTING_TARGET_PLATFORMS
        }
        ParseErrorCode::MetadataTargetNotAllowed => {
            DiagnosticCode::PARSE_METADATA_TARGET_NOT_ALLOWED
        }
        ParseErrorCode::MetadataFileScopeNotAllowed => {
            DiagnosticCode::PARSE_METADATA_FILE_SCOPE_NOT_ALLOWED
        }
        ParseErrorCode::UnsupportedLimitlessScope => {
            DiagnosticCode::PARSE_UNSUPPORTED_LIMITLESS_SCOPE
        }
        ParseErrorCode::DuplicateMetadata => DiagnosticCode::PARSE_DUPLICATE_METADATA,
        ParseErrorCode::FaultRequiresTest => DiagnosticCode::PARSE_FAULT_REQUIRES_TEST,
        ParseErrorCode::TestMetadataRequiresVerb => {
            DiagnosticCode::PARSE_TEST_METADATA_REQUIRES_VERB
        }
        ParseErrorCode::MalformedVerbContract => DiagnosticCode::PARSE_MALFORMED_VERB_CONTRACT,
        ParseErrorCode::OpenImportRequiresBlock => DiagnosticCode::PARSE_OPEN_IMPORT_REQUIRES_BLOCK,
        ParseErrorCode::MustUseRequiresReturn => DiagnosticCode::PARSE_MUST_USE_REQUIRES_RETURN,
        ParseErrorCode::UnsupportedEffect => DiagnosticCode::PARSE_UNSUPPORTED_EFFECT,
        ParseErrorCode::InvalidBoundedLimit => DiagnosticCode::PARSE_INVALID_BOUNDED_LIMIT,
        ParseErrorCode::InvalidStackLimit => DiagnosticCode::PARSE_INVALID_STACK_LIMIT,
        ParseErrorCode::UnsupportedCapability => DiagnosticCode::PARSE_UNSUPPORTED_CAPABILITY,
        ParseErrorCode::UnsupportedRepresentation => {
            DiagnosticCode::PARSE_UNSUPPORTED_REPRESENTATION
        }
    }
}

pub(crate) fn semantic_code(kind: &SemanticErrorKind) -> &'static str {
    packing::code(kind)
        .or_else(|| role_semantic_code(kind))
        .or_else(|| type_semantic_code(kind))
        .or_else(|| ownership_semantic_code(kind))
        .or_else(|| declaration_semantic_code(kind))
        .or_else(|| expression_semantic_code(kind))
        .unwrap_or_else(|| unreachable!("extended semantic diagnostic code handled above"))
        .as_str()
}

fn declaration_semantic_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::DuplicateVerbName { .. } => DiagnosticCode::DUPLICATE_VERB_NAME,
        SemanticErrorKind::DuplicateStructName { .. } => DiagnosticCode::DUPLICATE_STRUCT_NAME,
        SemanticErrorKind::DuplicatePackName { .. } => DiagnosticCode::DUPLICATE_PACK_NAME,
        SemanticErrorKind::InvalidSerializationContract { .. } => {
            DiagnosticCode::INVALID_SERIALIZATION_CONTRACT
        }
        SemanticErrorKind::EmptyEnum { .. } => DiagnosticCode::EMPTY_ENUM,
        SemanticErrorKind::DuplicateConstantName { .. } => DiagnosticCode::DUPLICATE_CONSTANT_NAME,
        SemanticErrorKind::ConstantCycle { .. } => DiagnosticCode::CONSTANT_CYCLE,
        SemanticErrorKind::DuplicateStructField { .. } => DiagnosticCode::DUPLICATE_STRUCT_FIELD,
        SemanticErrorKind::UnknownStructField { .. } => DiagnosticCode::UNKNOWN_STRUCT_FIELD,
        SemanticErrorKind::MissingStructField { .. } => DiagnosticCode::MISSING_STRUCT_FIELD,
        SemanticErrorKind::StructFieldTypeMismatch { .. } => {
            DiagnosticCode::STRUCT_FIELD_TYPE_MISMATCH
        }
        SemanticErrorKind::InvalidStructFieldRole { .. } => {
            DiagnosticCode::INVALID_STRUCT_FIELD_ROLE
        }
        SemanticErrorKind::FrozenStructField { .. } => DiagnosticCode::FROZEN_STRUCT_FIELD,
        SemanticErrorKind::SelfAssignment { .. } => DiagnosticCode::SELF_ASSIGNMENT,
        SemanticErrorKind::InvalidReprC { .. } => DiagnosticCode::INVALID_REPR_C,
        _ => return None,
    })
}

fn expression_semantic_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    expression_value_code(kind).or_else(|| expression_pattern_code(kind))
}

fn expression_value_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::TypeMismatch { .. } => DiagnosticCode::TYPE_MISMATCH,
        SemanticErrorKind::UnknownVerb { .. } => DiagnosticCode::UNKNOWN_VERB,
        SemanticErrorKind::UnresolvedResultConstructor { .. } => {
            DiagnosticCode::UNRESOLVED_RESULT_CONSTRUCTOR
        }
        SemanticErrorKind::NumericLiteralOutOfRange { .. } => {
            DiagnosticCode::NUMERIC_LITERAL_OUT_OF_RANGE
        }
        SemanticErrorKind::InvalidPrimitiveCast { .. } => DiagnosticCode::INVALID_PRIMITIVE_CAST,
        SemanticErrorKind::PrintIntegerTooWide { .. } => DiagnosticCode::PRINT_INTEGER_TOO_WIDE,
        SemanticErrorKind::PrimitiveCastOutOfRange { .. } => {
            DiagnosticCode::PRIMITIVE_CAST_OUT_OF_RANGE
        }
        SemanticErrorKind::ConstantRemainderByZero => DiagnosticCode::CONSTANT_REMAINDER_BY_ZERO,
        SemanticErrorKind::ConstantArithmeticOverflow { .. } => {
            DiagnosticCode::CONSTANT_ARITHMETIC_OVERFLOW
        }
        SemanticErrorKind::ConstantShiftCountOutOfRange { .. } => {
            DiagnosticCode::CONSTANT_SHIFT_COUNT_OUT_OF_RANGE
        }
        SemanticErrorKind::MixedIntegerOperands { .. } => {
            DiagnosticCode::OPERANDS_OF_DIFFERENT_INTEGER_TYPES
        }
        SemanticErrorKind::VoidValue { .. } => DiagnosticCode::VOID_VALUE,
        SemanticErrorKind::AssignToConstant { .. } => DiagnosticCode::ASSIGN_TO_CONSTANT,
        SemanticErrorKind::ConstantRuntimeDependency { .. } => {
            DiagnosticCode::CONSTANT_RUNTIME_DEPENDENCY
        }
        SemanticErrorKind::InvalidArenaCapacity { .. } => DiagnosticCode::INVALID_ARENA_CAPACITY,
        SemanticErrorKind::ArenaReferenceEscape { .. } => DiagnosticCode::ARENA_REFERENCE_ESCAPE,
        SemanticErrorKind::CrossArenaReference { .. } => DiagnosticCode::CROSS_ARENA_REFERENCE,
        SemanticErrorKind::ArenaReferenceLive { .. }
        | SemanticErrorKind::ArenaReferenceReleased { .. } => DiagnosticCode::ARENA_REFERENCE_LIVE,
        SemanticErrorKind::ReturnTypeMismatch { .. } => DiagnosticCode::RETURN_TYPE_MISMATCH,
        SemanticErrorKind::BindingTypeMismatch { .. } => DiagnosticCode::BINDING_TYPE_MISMATCH,
        SemanticErrorKind::MissingReturnValue => DiagnosticCode::MISSING_RETURN_VALUE,
        SemanticErrorKind::InvalidFieldAssignmentTarget { .. } => {
            DiagnosticCode::INVALID_FIELD_ASSIGNMENT_TARGET
        }
        SemanticErrorKind::FieldBorrowConflict { .. } => DiagnosticCode::FIELD_BORROW_CONFLICT,
        _ => return None,
    })
}

fn expression_pattern_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::UnknownMethod { .. } => DiagnosticCode::UNKNOWN_METHOD,
        SemanticErrorKind::InvalidReceiver { .. } => DiagnosticCode::INVALID_RECEIVER,
        SemanticErrorKind::ReceiverTypeMismatch { .. } => DiagnosticCode::RECEIVER_TYPE_MISMATCH,
        SemanticErrorKind::UnknownEnumVariant { .. } => DiagnosticCode::UNKNOWN_ENUM_VARIANT,
        SemanticErrorKind::EnumVariantArgumentCount { .. } => {
            DiagnosticCode::ENUM_VARIANT_ARGUMENT_COUNT
        }
        SemanticErrorKind::EnumVariantArgumentName { .. } => {
            DiagnosticCode::ENUM_VARIANT_ARGUMENT_NAME
        }
        SemanticErrorKind::EnumVariantArgumentTypeMismatch { .. } => {
            DiagnosticCode::ENUM_VARIANT_ARGUMENT_TYPE_MISMATCH
        }
        SemanticErrorKind::RecursiveType { .. } => DiagnosticCode::RECURSIVE_TYPE,
        SemanticErrorKind::NonExhaustiveMatch { .. } => DiagnosticCode::NON_EXHAUSTIVE_MATCH,
        SemanticErrorKind::UnreachablePattern { .. } => DiagnosticCode::UNREACHABLE_PATTERN,
        SemanticErrorKind::DuplicatePattern { .. } => DiagnosticCode::DUPLICATE_PATTERN,
        SemanticErrorKind::PatternTypeMismatch { .. } => DiagnosticCode::PATTERN_TYPE_MISMATCH,
        _ => return None,
    })
}

fn ownership_semantic_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    ownership_binding_code(kind).or_else(|| ownership_call_code(kind))
}

fn ownership_binding_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::DuplicateBinding { .. } => DiagnosticCode::DUPLICATE_BINDING,
        SemanticErrorKind::ShadowedBinding { .. } => DiagnosticCode::SHADOWED_BINDING,
        SemanticErrorKind::UndeclaredIdentifier { .. } => DiagnosticCode::UNDECLARED_IDENTIFIER,
        SemanticErrorKind::InvalidBorrowTarget { .. } => DiagnosticCode::INVALID_BORROW_TARGET,
        SemanticErrorKind::UseAfterMove { .. } => DiagnosticCode::USE_AFTER_MOVE,
        SemanticErrorKind::UseAfterDrop { .. } => DiagnosticCode::USE_AFTER_DROP,
        SemanticErrorKind::DoubleDrop { .. } => DiagnosticCode::DOUBLE_DROP,
        SemanticErrorKind::DropBorrow { .. } => DiagnosticCode::DROP_BORROW,
        SemanticErrorKind::DropFrozen { .. } => DiagnosticCode::DROP_FROZEN,
        SemanticErrorKind::InvalidDatArgument { .. } => DiagnosticCode::INVALID_DAT_ARGUMENT,
        SemanticErrorKind::ZeroOnDropFieldMove { .. } => DiagnosticCode::ZERO_ON_DROP_FIELD_MOVE,
        SemanticErrorKind::MoveFrozen { .. } | SemanticErrorKind::MutateFrozen { .. } => {
            DiagnosticCode::MOVE_OR_MUTATE_FROZEN
        }
        SemanticErrorKind::UnknownParameter { .. } => DiagnosticCode::UNKNOWN_PARAMETER,
        SemanticErrorKind::DuplicateArgument { .. } => DiagnosticCode::DUPLICATE_ARGUMENT,
        SemanticErrorKind::MixedArgumentModes { .. } => DiagnosticCode::MIXED_ARGUMENT_MODES,
        SemanticErrorKind::WrongArgumentCount { .. } => DiagnosticCode::WRONG_ARGUMENT_COUNT,
        SemanticErrorKind::InvalidArgumentRole { .. } => DiagnosticCode::INVALID_ARGUMENT_ROLE,
        _ => return None,
    })
}

fn ownership_call_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::InvalidAbsReturnOrigin { .. } => {
            DiagnosticCode::INVALID_ABS_RETURN_ORIGIN
        }
        SemanticErrorKind::InvalidIntrinsicArgument { .. } => {
            DiagnosticCode::INVALID_INTRINSIC_ARGUMENT
        }
        SemanticErrorKind::AmbiguousPositionalCall { .. } => {
            DiagnosticCode::AMBIGUOUS_POSITIONAL_CALL
        }
        SemanticErrorKind::BorrowedReturn { .. } => DiagnosticCode::BORROWED_RETURN,
        SemanticErrorKind::InvalidOwnerInitializer { .. } => {
            DiagnosticCode::INVALID_OWNER_INITIALIZER
        }
        SemanticErrorKind::LoopControlOutsideLoop { .. } => {
            DiagnosticCode::LOOP_CONTROL_OUTSIDE_LOOP
        }
        SemanticErrorKind::ReservedIntrinsicName { .. } => DiagnosticCode::RESERVED_INTRINSIC_NAME,
        SemanticErrorKind::ExclusiveLoanAlias { .. } => DiagnosticCode::EXCLUSIVE_LOAN_ALIAS,
        SemanticErrorKind::EscapingLoan { .. } => DiagnosticCode::ESCAPING_LOAN,
        SemanticErrorKind::MustUseIgnored { .. } => DiagnosticCode::MUST_USE_IGNORED,
        SemanticErrorKind::UndeclaredEffect { .. } => DiagnosticCode::UNDECLARED_EFFECT,
        SemanticErrorKind::UndeclaredCapability { .. } => DiagnosticCode::UNDECLARED_CAPABILITY,
        SemanticErrorKind::UnsupportedCapability { .. } => DiagnosticCode::UNSUPPORTED_CAPABILITY,
        SemanticErrorKind::NondeterministicCall { .. } => DiagnosticCode::NONDETERMINISTIC_CALL,
        SemanticErrorKind::DeterministicFloatingPoint => {
            DiagnosticCode::DETERMINISTIC_FLOATING_POINT
        }
        SemanticErrorKind::RealtimeEffect { .. } => DiagnosticCode::REALTIME_EFFECT,
        SemanticErrorKind::RealtimeUnboundedLoop => DiagnosticCode::REALTIME_UNBOUNDED_LOOP,
        SemanticErrorKind::RealtimeFloatingPoint => DiagnosticCode::REALTIME_FLOATING_POINT,
        SemanticErrorKind::RealtimeUnboundedRecursion => {
            DiagnosticCode::REALTIME_UNBOUNDED_RECURSION
        }
        SemanticErrorKind::StackLimitRecursion => DiagnosticCode::STACK_LIMIT_RECURSION,
        SemanticErrorKind::MissingStackLimitClaim { .. } => {
            DiagnosticCode::MISSING_STACK_LIMIT_CLAIM
        }
        SemanticErrorKind::MissingImportedStackLimitClaim { .. } => {
            DiagnosticCode::MISSING_IMPORTED_STACK_LIMIT_CLAIM
        }
        SemanticErrorKind::UnsupportedStackLimitTarget { .. } => {
            DiagnosticCode::UNSUPPORTED_STACK_LIMIT_TARGET
        }
        SemanticErrorKind::OpenStackLimitDispatch { .. } => {
            DiagnosticCode::OPEN_STACK_LIMIT_DISPATCH
        }
        SemanticErrorKind::BoundedBudgetExceeded { .. } => DiagnosticCode::BOUNDED_BUDGET_EXCEEDED,
        SemanticErrorKind::RoleMethodMustUseMismatch { .. } => {
            DiagnosticCode::ROLE_METHOD_MUST_USE_MISMATCH
        }
        _ => return None,
    })
}

fn type_semantic_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::UnknownType { .. } => DiagnosticCode::UNKNOWN_TYPE,
        SemanticErrorKind::UnknownTypeParameter { .. } => DiagnosticCode::UNKNOWN_TYPE_PARAMETER,
        SemanticErrorKind::GenericArityMismatch { .. } => DiagnosticCode::GENERIC_ARITY_MISMATCH,
        SemanticErrorKind::GenericConstraintMismatch { .. } => {
            DiagnosticCode::GENERIC_CONSTRAINT_MISMATCH
        }
        SemanticErrorKind::InvalidMutation { .. } => DiagnosticCode::INVALID_MUTATION,
        SemanticErrorKind::InvalidArrayCapacity { .. } => DiagnosticCode::INVALID_ARRAY_CAPACITY,
        SemanticErrorKind::InvalidRegionElementType { .. } => {
            DiagnosticCode::INVALID_REGION_ELEMENT_TYPE
        }
        SemanticErrorKind::InvalidIndexType { .. } => DiagnosticCode::INVALID_INDEX_TYPE,
        SemanticErrorKind::IndexOutOfBounds { .. } => DiagnosticCode::INDEX_OUT_OF_BOUNDS,
        SemanticErrorKind::NonIndexableTarget { .. } => DiagnosticCode::NON_INDEXABLE_TARGET,
        SemanticErrorKind::IndexedElementTypeMismatch { .. } => {
            DiagnosticCode::INDEXED_ELEMENT_TYPE_MISMATCH
        }
        SemanticErrorKind::UnresolvedResultConstructor { .. } => {
            DiagnosticCode::UNRESOLVED_RESULT_CONSTRUCTOR
        }
        SemanticErrorKind::InvalidCaseRole { .. } => DiagnosticCode::INVALID_CASE_ROLE,
        SemanticErrorKind::InvalidGuardAccess { .. } => DiagnosticCode::INVALID_GUARD_ACCESS,
        SemanticErrorKind::GuardTypeMismatch { .. }
        | SemanticErrorKind::ConditionTypeMismatch { .. } => {
            DiagnosticCode::GUARD_OR_CONDITION_TYPE_MISMATCH
        }
        SemanticErrorKind::BranchStateMismatch { .. } => DiagnosticCode::BRANCH_STATE_MISMATCH,
        SemanticErrorKind::DynamicRequiresAbs { .. } => DiagnosticCode::DYNAMIC_REQUIRES_ABS,
        SemanticErrorKind::UnknownDynamicRole { .. } => DiagnosticCode::UNKNOWN_DYNAMIC_ROLE,
        SemanticErrorKind::DynamicRoleMismatch { .. } => DiagnosticCode::DYNAMIC_ROLE_MISMATCH,
        _ => return None,
    })
}

fn role_semantic_code(kind: &SemanticErrorKind) -> Option<DiagnosticCode> {
    Some(match kind {
        SemanticErrorKind::DuplicateRoleName { .. } => DiagnosticCode::DUPLICATE_ROLE_NAME,
        SemanticErrorKind::UnknownRole { .. } => DiagnosticCode::UNKNOWN_ROLE,
        SemanticErrorKind::DuplicateRoleMethod { .. } => DiagnosticCode::DUPLICATE_ROLE_METHOD,
        SemanticErrorKind::MissingRoleMethod { .. } => DiagnosticCode::MISSING_ROLE_METHOD,
        SemanticErrorKind::RoleMethodMismatch { .. } => DiagnosticCode::ROLE_METHOD_MISMATCH,
        SemanticErrorKind::RoleMethodEffectMismatch { .. } => {
            DiagnosticCode::ROLE_METHOD_EFFECT_MISMATCH
        }
        SemanticErrorKind::RoleMethodCapabilityMismatch { .. } => {
            DiagnosticCode::ROLE_METHOD_CAPABILITY_MISMATCH
        }
        SemanticErrorKind::RoleMethodRealtimeMismatch { .. } => {
            DiagnosticCode::ROLE_METHOD_REALTIME_MISMATCH
        }
        SemanticErrorKind::RoleMethodDeterministicMismatch { .. } => {
            DiagnosticCode::ROLE_METHOD_DETERMINISTIC_MISMATCH
        }
        SemanticErrorKind::InvalidRoleReceiver { .. } => DiagnosticCode::INVALID_ROLE_RECEIVER,
        _ => return None,
    })
}
