use std::collections::HashMap;

use crate::ast::{GenericParam, GenericParamKind, TypeName};
use crate::lexer::SourceSpan;

use super::errors::{SemanticError, SemanticErrorKind};

/// A deterministic mapping from declaration parameters to concrete type arguments.
///
/// This is the semantic input for future monomorphization. It does not emit code
/// or allocate a backend representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeSubstitution {
    bindings: HashMap<String, TypeName>,
    const_bindings: HashMap<String, String>,
    call_bindings: HashMap<String, String>,
}

impl TypeSubstitution {
    pub(crate) fn for_type(
        type_name: &str,
        parameters: &[GenericParam],
        arguments: &[TypeName],
        span: SourceSpan,
    ) -> Result<Self, SemanticError> {
        if parameters.len() != arguments.len() {
            return Err(SemanticError {
                kind: SemanticErrorKind::GenericArityMismatch {
                    name: type_name.to_owned(),
                    expected: parameters.len(),
                    found: arguments.len(),
                },
                span,
            });
        }
        let mut bindings = HashMap::new();
        let mut const_bindings = HashMap::new();
        for (parameter, argument) in parameters.iter().zip(arguments) {
            match &parameter.kind {
                GenericParamKind::Type => {
                    bindings.insert(parameter.name.clone(), argument.clone());
                }
                GenericParamKind::Const { .. } => {
                    const_bindings.insert(parameter.name.clone(), argument.name.clone());
                }
            }
        }
        Ok(Self { bindings, const_bindings, call_bindings: HashMap::new() })
    }

    pub(crate) fn with_call_bindings(mut self, call_bindings: &HashMap<String, String>) -> Self {
        self.call_bindings = call_bindings.clone();
        self
    }

    pub(crate) fn apply_call(&self, name: &str) -> Option<&str> {
        self.call_bindings.get(name).map(String::as_str)
    }

    pub(crate) fn apply_const(&self, name: &str) -> Option<&str> {
        self.const_bindings.get(name).map(String::as_str)
    }

    // Used by the forthcoming monomorphization pass after concrete instances
    // become reachable from the semantic type graph.
    #[allow(dead_code)]
    pub(crate) fn apply(&self, type_name: &TypeName) -> TypeName {
        if let Some(argument) = self.bindings.get(&type_name.name) {
            return Self::replace_root(argument, type_name.span);
        }
        if let Some(value) = self.apply_const(&type_name.name) {
            return TypeName {
                name: value.to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span: type_name.span,
            };
        }
        TypeName {
            name: type_name.name.clone(),
            arguments: type_name.arguments.iter().map(|argument| self.apply(argument)).collect(),
            reference_role: type_name.reference_role.clone(),
            span: type_name.span,
        }
    }

    fn replace_root(type_name: &TypeName, span: SourceSpan) -> TypeName {
        TypeName { span, ..type_name.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::TypeSubstitution;
    use crate::ast::{GenericParam, TypeName};
    use crate::lexer::SourceSpan;

    fn ty(name: &str) -> TypeName {
        TypeName {
            name: name.to_owned(),
            arguments: Vec::new(),
            reference_role: None,
            span: SourceSpan::new(0, 1),
        }
    }

    #[test]
    fn substitutes_nested_type_applications() {
        let parameters = vec![GenericParam {
            name: "T".to_owned(),
            kind: crate::ast::GenericParamKind::Type,
            bound: None,
            bounds: Vec::new(),
            span: ty("T").span,
        }];
        let substitution =
            TypeSubstitution::for_type("Box", &parameters, &[ty("Int")], ty("Box").span)
                .expect("arity should match");
        let applied = TypeName {
            name: "Result".to_owned(),
            arguments: vec![
                ty("T"),
                TypeName {
                    name: "Array".to_owned(),
                    arguments: vec![ty("T")],
                    reference_role: None,
                    span: ty("Array").span,
                },
            ],
            reference_role: None,
            span: ty("Result").span,
        };
        assert_eq!(substitution.apply(&applied).arguments[0].name, "Int");
        assert_eq!(substitution.apply(&applied).arguments[1].arguments[0].name, "Int");
    }

    #[test]
    fn exposes_const_arguments_for_expression_specialization() {
        let parameters = vec![GenericParam {
            name: "N".to_owned(),
            kind: crate::ast::GenericParamKind::Const { domain: ty("Usize") },
            bound: None,
            bounds: Vec::new(),
            span: ty("N").span,
        }];
        let substitution =
            TypeSubstitution::for_type("Buffer", &parameters, &[ty("4")], ty("Buffer").span)
                .expect("arity should match");
        assert_eq!(substitution.apply_const("N"), Some("4"));
    }
}
