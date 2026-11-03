use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::generic_errors::arity_error;
use super::generics::ResolvedType;
use crate::ast::TypeName;

impl Analyzer {
    pub(super) fn resolve_volatile_address_type(
        &self,
        type_name: &TypeName,
    ) -> Result<ResolvedType, SemanticError> {
        if type_name.arguments.len() != 1 {
            return Err(arity_error(
                "VolatileAddress",
                1,
                type_name.arguments.len(),
                type_name.span,
            ));
        }
        let target = &type_name.arguments[0];
        if self.is_generic_parameter(&target.name) {
            if !target.arguments.is_empty() {
                return Err(arity_error(&target.name, 0, target.arguments.len(), target.span));
            }
            return Ok(ResolvedType::Applied {
                name: "VolatileAddress".to_owned(),
                arguments: vec![ResolvedType::GenericParameter(target.name.clone())],
            });
        }
        let Some(structure) = self.struct_types.get(&target.name) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownType { name: target.name.clone() },
                span: target.span,
            });
        };
        if !structure.is_volatile() {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidReprC {
                    declaration: "VolatileAddress".to_owned(),
                    reason: format!(
                        "address target `{}` must be declared `meta volatile`",
                        target.name
                    ),
                },
                span: target.span,
            });
        }
        Ok(ResolvedType::Applied {
            name: "VolatileAddress".to_owned(),
            arguments: vec![ResolvedType::Concrete(target.name.clone())],
        })
    }
}
