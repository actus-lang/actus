use crate::ast::{Program, Role, TopLevelDecl};

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(crate) fn validate_method_declarations(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration else { continue };
            let Some(receiver) = verb.params.first() else { continue };
            if receiver.name != "self" {
                continue;
            }
            if !matches!(receiver.role, Role::Erg | Role::Abs | Role::Dat | Role::Ins)
                || !self.struct_types.contains_key(&receiver.ty.name)
            {
                return Err(SemanticError {
                    kind: SemanticErrorKind::InvalidReceiver { method: verb.name.clone() },
                    span: receiver.span,
                });
            }
        }
        Ok(())
    }
}
