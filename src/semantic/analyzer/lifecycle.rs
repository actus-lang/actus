use super::super::errors::SemanticError;
use super::Analyzer;
use crate::ast::Program;

impl Analyzer {
    pub(super) fn new() -> Self {
        Self::default()
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
        self.register_constants(program)?;
        self.register_declarations(program)?;
        self.collect_dynamic_roles(program);
        self.validate_method_declarations(program)?;
        self.analyze_verbs(program)?;
        Ok(self.finalize_model())
    }

    fn finalize_model(mut self) -> super::super::model::SemanticModel {
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
        self.model
    }
}
