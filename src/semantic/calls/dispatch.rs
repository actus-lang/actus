use crate::ast::{Argument, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::signatures;

impl Analyzer {
    pub(in crate::semantic) fn visit_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        resolved_type: Option<&TypeName>,
        generic_arguments: &[TypeName],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.visit_scalar_copy_call(callee, arguments, span)? {
            return Ok(());
        }
        if matches!(callee, "print" | "eprint")
            && print_uses_label(arguments)
            && !signatures::has_matching_named_signature(self.signatures.get(callee), arguments)
        {
            return self.visit_intrinsic_call(callee, arguments, span).map(|_| ());
        }
        if let Some(result) =
            self.visit_special_call(callee, arguments, resolved_type, generic_arguments, span)?
        {
            return result;
        }
        self.visit_regular_call(callee, arguments, resolved_type, generic_arguments, span)
    }

    fn visit_regular_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        resolved_type: Option<&TypeName>,
        generic_arguments: &[TypeName],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let mut explicit_type_application = resolved_type.cloned().or_else(|| {
            (!generic_arguments.is_empty()).then(|| {
                TypeName::unresolved(
                    callee.split_once('[').map_or(callee, |(base, _)| base),
                    generic_arguments.to_vec(),
                    None,
                    span,
                )
            })
        });
        let parsed_lookup_name = explicit_type_application
            .as_ref()
            .map_or(callee, |type_name| type_name.name.as_str())
            .to_owned();
        let lookup_name = self.qualified_call_lookup_name(&parsed_lookup_name, span)?;
        let Some(mut signature) = self.signatures.get(&lookup_name).cloned() else {
            for argument in arguments {
                self.visit_expression(&argument.expression)?;
            }
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownVerb { name: lookup_name.clone() },
                span,
            });
        };
        if !signature.generic_parameters.is_empty() {
            if let Some(type_name) = explicit_type_application.as_mut() {
                resolve_generic_call_types(type_name, &signature, &lookup_name);
            }
            signature = self.instantiate_generic_signature(
                &lookup_name,
                &signature,
                arguments,
                explicit_type_application.as_ref().map(|type_name| type_name.arguments.as_slice()),
                span,
            )?;
        } else if let Some(type_name) = explicit_type_application {
            return Err(SemanticError {
                kind: SemanticErrorKind::GenericArityMismatch {
                    name: lookup_name,
                    expected: 0,
                    found: type_name.arguments.len(),
                },
                span: type_name.span,
            });
        }
        self.visit_call_with_signature(&lookup_name, arguments, span, &signature)
    }

    fn visit_scalar_copy_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let specialized = callee.starts_with("copy__")
            && arguments.len() == 1
            && arguments[0].name.as_deref() == Some("value");
        let intrinsic = specialized || self.should_use_scalar_copy_intrinsic(callee, arguments);
        if intrinsic {
            self.visit_intrinsic_call("copy", arguments, span)?;
        }
        Ok(intrinsic)
    }

    fn should_use_scalar_copy_intrinsic(&self, callee: &str, arguments: &[Argument]) -> bool {
        if callee != "copy" || arguments.len() != 1 || arguments[0].name.as_deref() != Some("value")
        {
            return false;
        }
        if !self.local_signatures.contains(callee) {
            return true;
        }
        self.signatures
            .get(callee)
            .is_none_or(|signature| signature.params.len() != 1 || signature.params[0].0 != "value")
    }
}

fn print_uses_label(arguments: &[Argument]) -> bool {
    arguments.iter().any(|argument| {
        matches!(
            argument.name.as_deref(),
            Some("value")
                | Some("format")
                | Some("string")
                | Some("int")
                | Some("byte")
                | Some("bool")
                | Some("buffer")
        )
    })
}

fn resolve_generic_call_types(
    type_name: &mut TypeName,
    signature: &super::VerbSignature,
    lookup_name: &str,
) {
    for argument in &mut type_name.arguments {
        resolve_generic_call_types(argument, signature, lookup_name);
    }
    if type_name.resolved.is_some() {
        return;
    }
    if crate::ast::lookup_builtin_type(&type_name.name).is_some()
        || crate::ast::primitive_type(&type_name.name).is_some()
    {
        *type_name = TypeName::builtin(
            type_name.name.clone(),
            type_name.arguments.clone(),
            type_name.reference_role.clone(),
            type_name.span,
        );
    } else if signature.generic_parameters.iter().any(|parameter| parameter.name == type_name.name)
    {
        *type_name = TypeName::resolved(
            crate::ast::DeclarationIdentity::generic_parameter(lookup_name, type_name.name.clone()),
            type_name.name.clone(),
            type_name.arguments.clone(),
            type_name.reference_role.clone(),
            type_name.span,
        );
    }
}
