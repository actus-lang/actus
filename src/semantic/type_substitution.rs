use std::collections::HashMap;

use crate::ast::{GenericParam, TypeName};
use crate::lexer::SourceSpan;

use super::errors::{SemanticError, SemanticErrorKind};

/// A deterministic mapping from declaration parameters to concrete type arguments.
///
/// This is the semantic input for future monomorphization. It does not emit code
/// or allocate a backend representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeSubstitution {
    bindings: HashMap<String, TypeName>,
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
        let bindings = parameters
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
            .collect();
        Ok(Self { bindings })
    }

    // Used by the forthcoming monomorphization pass after concrete instances
    // become reachable from the semantic type graph.
    #[allow(dead_code)]
    pub(crate) fn apply(&self, type_name: &TypeName) -> TypeName {
        if let Some(argument) = self.bindings.get(&type_name.name) {
            return Self::replace_root(argument, type_name.span);
        }
        TypeName {
            name: type_name.name.clone(),
            arguments: type_name.arguments.iter().map(|argument| self.apply(argument)).collect(),
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
        TypeName { name: name.to_owned(), arguments: Vec::new(), span: SourceSpan::new(0, 1) }
    }

    #[test]
    fn substitutes_nested_type_applications() {
        let parameters = vec![GenericParam {
            name: "T".to_owned(),
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
                    span: ty("Array").span,
                },
            ],
            span: ty("Result").span,
        };
        assert_eq!(substitution.apply(&applied).arguments[0].name, "Int");
        assert_eq!(substitution.apply(&applied).arguments[1].arguments[0].name, "Int");
    }
}
