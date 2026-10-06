use crate::ast::{BuiltinType, Role, TypeIdentity, TypeName};
use crate::lexer::SourceSpan;

use super::state::{AccessState, OwnershipState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub name: String,
    pub role: Role,
    pub ty: Option<BuiltinType>,
    pub span: SourceSpan,
    pub ownership: OwnershipState,
    pub access: AccessState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BorrowRecord {
    pub id: usize,
    pub owner: String,
    pub field: Option<String>,
    pub scope_depth: usize,
    pub origin_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExclusiveLoan {
    pub id: usize,
    pub owner: String,
    pub callee: String,
    pub parameter: String,
    pub origin_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Origin {
    None,
    AbsParameter { parameter_index: usize },
    Derived { root_parameter: usize },
    Binding { binding_index: usize },
    Unknown,
    Multiple { roots: Vec<OriginRoot> },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OriginRoot {
    Parameter(usize),
    Binding(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginRecord {
    pub span: SourceSpan,
    pub origin: Origin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgumentRoleSource {
    Explicit,
    Inferred,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentRoleFact {
    pub callee: String,
    pub parameter: String,
    pub role: Role,
    pub source: ArgumentRoleSource,
    pub call_span: SourceSpan,
    pub argument_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiteralFact {
    pub span: SourceSpan,
    pub kind: &'static str,
    pub value: String,
    pub suffix: Option<String>,
    pub type_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalFact {
    pub span: SourceSpan,
    pub condition_span: SourceSpan,
    pub condition_type: String,
    pub then_type: Option<String>,
    pub else_type: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericInstance {
    pub name: String,
    pub arguments: Vec<TypeName>,
    pub canonical_key: String,
    pub caller: Option<String>,
    pub call_span: SourceSpan,
}

impl GenericInstance {
    pub fn identity(&self) -> TypeIdentity {
        TypeIdentity::from_type_name(&TypeName {
            name: self.name.clone(),
            arguments: self.arguments.clone(),
            reference_role: None,
            span: self.call_span,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackLayoutContract {
    pub name: String,
    pub identity: String,
    pub storage: String,
    pub storage_bytes: u64,
    pub storage_bits: u64,
    pub alignment_bytes: u64,
    pub endianness: String,
    pub fields: Vec<PackFieldContract>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackFieldContract {
    pub name: String,
    pub role: String,
    pub ty: String,
    pub offset: u16,
    pub width: u16,
    pub indexed_count: Option<u32>,
    pub has_default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializationContract {
    pub name: String,
    pub source_type: String,
    pub encode_name: String,
    pub decode_name: String,
    pub validate_name: String,
    pub migrate_name: String,
    pub endianness: String,
    pub version_offset: u16,
    pub payload_offset: u16,
    pub payload_length: u16,
    pub checksum_start: u16,
    pub checksum_end: u16,
    pub checksum_offset: u16,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReachablePerformance {
    pub role_name: String,
    pub target_type: String,
    pub method_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FatPointerLayout {
    pub data_ptr_word: u8,
    pub vtable_ptr_word: u8,
    pub word_count: u8,
    pub alignment_words: u8,
}

impl FatPointerLayout {
    pub const DYNAMIC_ROLE: Self =
        Self { data_ptr_word: 0, vtable_ptr_word: 1, word_count: 2, alignment_words: 1 };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicRoleType {
    pub role_name: String,
    pub layout: FatPointerLayout,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SemanticModel {
    pub bindings: Vec<Binding>,
    pub borrows: Vec<BorrowRecord>,
    pub exclusive_loans: Vec<ExclusiveLoan>,
    pub expression_origins: Vec<OriginRecord>,
    pub argument_roles: Vec<ArgumentRoleFact>,
    pub cleanup_plans: Vec<super::cleanup::ScopeCleanup>,
    pub return_unwind_plans: Vec<super::cleanup::UnwindPlan>,
    pub loop_unwind_plans: Vec<super::cleanup::LoopUnwindPlan>,
    pub generic_instances: Vec<GenericInstance>,
    pub pack_layouts: Vec<PackLayoutContract>,
    pub serialization_contracts: Vec<SerializationContract>,
    pub reachable_performances: Vec<ReachablePerformance>,
    pub dynamic_roles: Vec<DynamicRoleType>,
    pub drop_types: Vec<String>,
    pub binding_type_names: std::collections::HashMap<usize, TypeName>,
    pub arena_provenance: std::collections::HashMap<usize, usize>,
    pub literal_facts: Vec<LiteralFact>,
    pub conditional_facts: Vec<ConditionalFact>,
}
