use crate::ast::{DispatchMode, Program, TopLevelDecl};

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn register_declarations(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let (name, params, return_type, span, signature) = match declaration {
                TopLevelDecl::Verb(verb) => {
                    (&verb.name, &verb.params, &verb.return_type, verb.span, verb.signature())
                }
                TopLevelDecl::ExternalVerb(verb) => {
                    (&verb.name, &verb.params, &verb.return_type, verb.span, verb.signature())
                }
                TopLevelDecl::Struct(_)
                | TopLevelDecl::Pack(_)
                | TopLevelDecl::Enum(_)
                | TopLevelDecl::Role(_)
                | TopLevelDecl::Perform(_)
                | TopLevelDecl::OpenSibling(_)
                | TopLevelDecl::Import(_) => continue,
            };
            let generic_parameters = match declaration {
                TopLevelDecl::Verb(verb) => &verb.generic_parameters,
                TopLevelDecl::ExternalVerb(verb) => &verb.generic_parameters,
                _ => unreachable!(),
            };
            self.with_generic_scope(generic_parameters, |analyzer| {
                for parameter in params {
                    analyzer.validate_dynamic_parameter(parameter)?;
                    if parameter.dispatch == DispatchMode::Static {
                        analyzer.validate_type_reference(&parameter.ty)?;
                    }
                }
                if let Some(return_type) = return_type {
                    analyzer.validate_type_reference(&return_type.ty)?;
                }
                Ok(())
            })?;
            if super::super::intrinsics::is_reserved_name(name) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::ReservedIntrinsicName { name: name.clone() },
                    span,
                });
            }
            if self.signatures.contains_key(name) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateVerbName { name: name.clone() },
                    span,
                });
            }
            self.signatures.insert(name.clone(), signature);
        }
        Ok(())
    }

    pub(super) fn analyze_verbs(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            if let TopLevelDecl::Verb(verb) = declaration {
                let parameters = verb.generic_parameters.clone();
                self.with_generic_scope(&parameters, |analyzer| analyzer.analyze_verb_body(verb))?;
            }
        }
        for declaration in &program.declarations {
            if let TopLevelDecl::Perform(perform) = declaration {
                for method in &perform.methods {
                    let parameters = method.generic_parameters.clone();
                    self.with_generic_scope(&parameters, |analyzer| {
                        analyzer.analyze_verb_body(method)
                    })?;
                }
            }
        }
        Ok(())
    }
}
