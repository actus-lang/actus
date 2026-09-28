use std::collections::{HashMap, HashSet};

use super::super::errors::SemanticError;
use super::Analyzer;
use crate::ast::Program;

impl Analyzer {
    pub(super) fn new() -> Self {
        Self {
            model: super::super::model::SemanticModel {
                bindings: Vec::new(),
                borrows: Vec::new(),
                exclusive_loans: Vec::new(),
                expression_origins: Vec::new(),
                cleanup_plans: Vec::new(),
                return_unwind_plans: Vec::new(),
                loop_unwind_plans: Vec::new(),
                generic_instances: Vec::new(),
                reachable_performances: Vec::new(),
                dynamic_roles: Vec::new(),
                drop_types: Vec::new(),
                binding_type_names: HashMap::new(),
                arena_provenance: HashMap::new(),
            },
            scopes: Vec::new(),
            next_borrow_id: 0,
            next_loan_id: 0,
            active_borrow_ids: HashSet::new(),
            signatures: HashMap::new(),
            loop_boundaries: Vec::new(),
            current_return_type: None,
            current_return_type_name: None,
            current_return_is_builtin_result: false,
            current_return_access: None,
            current_abs_origins: HashMap::new(),
            binding_origins: HashMap::new(),
            struct_types: HashMap::new(),
            pack_types: HashMap::new(),
            enum_types: HashMap::new(),
            role_types: HashMap::new(),
            performances: HashSet::new(),
            performance_methods: HashMap::new(),
            performance_roles: HashMap::new(),
            reachable_performances: HashSet::new(),
            drop_types: HashSet::new(),
            binding_struct_types: HashMap::new(),
            binding_struct_type_applications: HashMap::new(),
            binding_type_names: HashMap::new(),
            binding_enum_types: HashMap::new(),
            binding_enum_type_applications: HashMap::new(),
            binding_dynamic_roles: HashMap::new(),
            binding_arena_provenance: HashMap::new(),
            binding_scope_depth: HashMap::new(),
            arena_scope_depth: HashMap::new(),
            field_arena_provenance: HashMap::new(),
            expression_arena_provenance: HashMap::new(),
            next_arena_id: 0,
            generic_scopes: Vec::new(),
            generic_bounds: HashMap::new(),
            generic_instances:
                super::super::generic_cache::GenericInstanceCache::for_current_toolchain(),
            expected_expression_type: None,
            inferred_expression_types: HashMap::new(),
            type_registry: super::super::types::TypeRegistry::new(),
        }
    }

    pub(super) fn analyze(
        mut self,
        program: &Program,
    ) -> Result<super::super::model::SemanticModel, SemanticError> {
        self.register_enums(program)?;
        self.register_roles(program)?;
        self.register_structs(program)?;
        self.validate_pack_declarations(program)?;
        self.validate_role_declarations()?;
        self.validate_performances(program)?;
        self.validate_recursive_types()?;
        self.register_declarations(program)?;
        self.collect_dynamic_roles(program);
        self.validate_method_declarations(program)?;
        self.analyze_verbs(program)?;
        self.model.generic_instances = self.generic_instances.into_instances();
        self.model.reachable_performances = self.reachable_performances.into_iter().collect();
        self.model.reachable_performances.sort_by(|left, right| {
            (&left.target_type, &left.role_name, &left.method_name).cmp(&(
                &right.target_type,
                &right.role_name,
                &right.method_name,
            ))
        });
        self.model.drop_types = self.drop_types.into_iter().collect();
        self.model.drop_types.sort();
        self.model.binding_type_names = self.binding_type_names.clone();
        self.model.arena_provenance = self.binding_arena_provenance.clone();
        self.current_return_type = None;
        self.current_return_type_name = None;
        self.current_return_access = None;
        Ok(self.model)
    }
}
