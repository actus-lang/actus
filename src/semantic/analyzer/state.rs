use std::collections::{HashMap, HashSet};

use crate::ast::{BuiltinType, EnumDef, ReturnAccess, RoleDecl, StructDef};

use super::super::model::{Origin, SemanticModel};

pub(crate) struct ScopeFrame {
    pub(crate) span: crate::lexer::SourceSpan,
    pub(crate) bindings: HashMap<String, usize>,
    pub(crate) borrow_ids: Vec<usize>,
    pub(crate) declaration_indices: Vec<usize>,
    pub(crate) payload_cleanup: Vec<(usize, String, String, String)>,
}

#[derive(Default)]
pub(crate) struct Analyzer {
    pub(crate) model: SemanticModel,
    pub(crate) scopes: Vec<ScopeFrame>,
    pub(crate) next_borrow_id: usize,
    pub(crate) next_loan_id: usize,
    pub(crate) active_borrow_ids: HashSet<usize>,
    pub(crate) signatures: HashMap<String, super::super::calls::VerbSignature>,
    pub(crate) constants: HashMap<String, crate::ast::TypeName>,
    pub(crate) constant_initializers: HashMap<String, crate::ast::Expr>,
    pub(crate) loop_boundaries: Vec<usize>,
    pub(crate) current_return_type: Option<BuiltinType>,
    pub(crate) current_return_type_name: Option<crate::ast::TypeName>,
    pub(crate) current_return_is_builtin_result: bool,
    pub(crate) current_return_access: Option<ReturnAccess>,
    pub(crate) current_abs_origins: HashMap<String, usize>,
    pub(crate) binding_origins: HashMap<usize, Origin>,
    pub(crate) struct_types: HashMap<String, StructDef>,
    pub(crate) pack_types: HashMap<String, crate::ast::PackDecl>,
    pub(crate) enum_types: HashMap<String, EnumDef>,
    pub(crate) role_types: HashMap<String, RoleDecl>,
    pub(crate) performances: HashSet<(String, String)>,
    pub(crate) performance_methods: HashMap<(String, String), super::super::calls::VerbSignature>,
    pub(crate) performance_roles: HashMap<(String, String), String>,
    pub(crate) reachable_performances: HashSet<super::super::model::ReachablePerformance>,
    pub(crate) drop_types: HashSet<String>,
    pub(crate) binding_struct_types: HashMap<usize, String>,
    pub(crate) binding_struct_type_applications: HashMap<usize, crate::ast::TypeName>,
    pub(crate) binding_type_names: HashMap<usize, crate::ast::TypeName>,
    pub(crate) binding_enum_types: HashMap<usize, String>,
    pub(crate) binding_enum_type_applications: HashMap<usize, crate::ast::TypeName>,
    pub(crate) binding_dynamic_roles: HashMap<usize, String>,
    pub(crate) binding_arena_provenance: HashMap<usize, usize>,
    pub(crate) binding_scope_depth: HashMap<usize, usize>,
    pub(crate) arena_scope_depth: HashMap<usize, usize>,
    pub(crate) field_arena_provenance: HashMap<(usize, String), usize>,
    pub(crate) expression_arena_provenance: HashMap<(usize, usize), usize>,
    pub(crate) next_arena_id: usize,
    pub(crate) generic_scopes: Vec<HashSet<String>>,
    pub(crate) generic_bounds: HashMap<String, Vec<String>>,
    pub(crate) generic_instances: super::super::generic_cache::GenericInstanceCache,
    pub(crate) expected_expression_type: Option<crate::ast::TypeName>,
    pub(crate) inferred_expression_types: HashMap<(usize, usize), crate::ast::TypeName>,
    pub(crate) type_registry: super::super::types::TypeRegistry,
}
