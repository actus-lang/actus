use crate::ast::{VerbDecl, lookup_builtin_type};

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::validation::block_guarantees_return;

impl Analyzer {
    pub(super) fn analyze_verb_body(&mut self, verb: &VerbDecl) -> Result<(), SemanticError> {
        self.next_expression_scope += 1;
        self.current_expression_scope = self.next_expression_scope;
        self.configure_return_state(verb);
        self.validate_realtime_signature(verb)?;
        self.initialize_origin_parameter_map(&verb.params);
        self.enter_scope(verb.body.span);
        self.bind_parameters(&verb.params)?;
        self.visit_block(&verb.body)?;
        self.validate_bounded_contract(verb)?;
        self.validate_missing_return(verb)?;
        self.leave_scope();
        Ok(())
    }

    fn configure_return_state(&mut self, verb: &VerbDecl) {
        self.current_effects = verb
            .metadata
            .iter()
            .filter_map(|attribute| match attribute {
                crate::ast::MetaAttribute::Effect(effect) => Some(*effect),
                _ => None,
            })
            .collect();
        self.current_capabilities = verb
            .metadata
            .iter()
            .filter_map(|attribute| match attribute {
                crate::ast::MetaAttribute::Capability(capability) => Some(*capability),
                _ => None,
            })
            .collect();
        self.current_deterministic = verb
            .metadata
            .iter()
            .any(|attribute| matches!(attribute, crate::ast::MetaAttribute::Deterministic));
        self.current_realtime = verb
            .metadata
            .iter()
            .any(|attribute| matches!(attribute, crate::ast::MetaAttribute::Realtime));
        self.current_bounded = verb.metadata.iter().find_map(|attribute| match attribute {
            crate::ast::MetaAttribute::Bounded(contract) => Some(contract.clone()),
            _ => None,
        });
        self.current_verb_name = Some(verb.name.clone());
        self.current_return_access =
            verb.return_type.as_ref().map(|return_type| return_type.access);
        self.current_return_type = verb
            .return_type
            .as_ref()
            .and_then(|return_type| lookup_builtin_type(&return_type.ty.name));
        self.current_return_type_name = verb
            .return_type
            .as_ref()
            .map(|return_type| self.normalize_imported_type_name(&return_type.ty));
        self.current_return_is_builtin_result =
            verb.return_type.as_ref().is_some_and(|return_type| {
                return_type.ty.name == "Result"
                    && self
                        .enum_types
                        .get("Result")
                        .is_some_and(|definition| definition.span.start == 0)
            });
    }

    pub(super) fn validate_realtime_signature(&self, verb: &VerbDecl) -> Result<(), SemanticError> {
        if !self.current_realtime {
            return Ok(());
        }
        if let Some(parameter) =
            verb.params.iter().find(|parameter| type_contains_float(&parameter.ty))
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::RealtimeFloatingPoint,
                span: parameter.span,
            });
        }
        if let Some(return_type) = &verb.return_type
            && type_contains_float(&return_type.ty)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::RealtimeFloatingPoint,
                span: return_type.span,
            });
        }
        Ok(())
    }

    fn bind_parameters(&mut self, parameters: &[crate::ast::Param]) -> Result<(), SemanticError> {
        for parameter in parameters {
            let ty = lookup_builtin_type(&parameter.ty.name);
            self.bind(parameter.role.clone(), parameter.name.clone(), ty, parameter.span)?;
            let index = self.binding(&parameter.name, parameter.span)?;
            self.binding_type_names.insert(index, parameter.ty.clone());
            if parameter.dispatch == crate::ast::DispatchMode::Dynamic {
                let index = self.binding(&parameter.name, parameter.span)?;
                self.binding_dynamic_roles.insert(index, parameter.ty.name.clone());
            }
            self.record_struct_binding(&parameter.name, &parameter.ty, parameter.span)?;
            self.record_enum_binding(&parameter.name, &parameter.ty, parameter.span)?;
            if let Ok(index) = self.binding(&parameter.name, parameter.span)
                && let Some(parameter_index) = self.current_abs_origins.get(&parameter.name)
            {
                self.binding_origins.insert(
                    index,
                    super::super::model::Origin::AbsParameter { parameter_index: *parameter_index },
                );
            }
        }
        Ok(())
    }

    fn validate_missing_return(&self, verb: &VerbDecl) -> Result<(), SemanticError> {
        let requires_return_value =
            self.current_return_type.is_some() || self.current_return_is_builtin_result;
        if requires_return_value && !block_guarantees_return(&verb.body) {
            return Err(SemanticError {
                kind: SemanticErrorKind::MissingReturnValue,
                span: verb.body.span,
            });
        }
        Ok(())
    }
}

fn type_contains_float(type_name: &crate::ast::TypeName) -> bool {
    matches!(type_name.name.as_str(), "f32" | "f64")
        || type_name.arguments.iter().any(type_contains_float)
}
