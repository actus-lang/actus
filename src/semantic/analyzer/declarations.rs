use crate::ast::{DispatchMode, GenericParam, Param, Program, ReturnType, TopLevelDecl};

use super::super::analyzer::Analyzer;
use super::super::calls::VerbSignature;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn register_declarations(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            self.register_declaration(declaration)?;
        }
        Ok(())
    }

    fn register_declaration(&mut self, declaration: &TopLevelDecl) -> Result<(), SemanticError> {
        match declaration {
            TopLevelDecl::Verb(verb) => self.register_signature(
                &verb.name,
                &verb.params,
                verb.return_type.as_ref(),
                &verb.generic_parameters,
                verb.span,
                verb.signature(),
            ),
            TopLevelDecl::ExternalVerb(verb) => self.register_signature(
                &verb.name,
                &verb.params,
                verb.return_type.as_ref(),
                &verb.generic_parameters,
                verb.span,
                verb.signature(),
            ),
            _ => Ok(()),
        }
    }

    fn register_signature(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: Option<&ReturnType>,
        generic_parameters: &[GenericParam],
        span: crate::lexer::SourceSpan,
        signature: VerbSignature,
    ) -> Result<(), SemanticError> {
        self.validate_signature_types(params, return_type, generic_parameters)?;
        if super::super::intrinsics::is_reserved_name(name) {
            return Err(SemanticError {
                kind: SemanticErrorKind::ReservedIntrinsicName { name: name.to_owned() },
                span,
            });
        }
        if self.signatures.contains_key(name) {
            return Err(SemanticError {
                kind: SemanticErrorKind::DuplicateVerbName { name: name.to_owned() },
                span,
            });
        }
        self.signatures.insert(name.to_owned(), signature);
        Ok(())
    }

    fn validate_signature_types(
        &mut self,
        params: &[Param],
        return_type: Option<&ReturnType>,
        generic_parameters: &[GenericParam],
    ) -> Result<(), SemanticError> {
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
        })
    }

    pub(super) fn analyze_verbs(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            if let TopLevelDecl::Verb(verb) = declaration {
                let parameters = verb.generic_parameters.clone();
                self.current_generic_owner = (!parameters.is_empty()).then(|| verb.name.clone());
                let result = self
                    .with_generic_scope(&parameters, |analyzer| analyzer.analyze_verb_body(verb));
                self.current_generic_owner = None;
                result?;
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
