use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Erg,
    Abs,
    Dat,
    Ins,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeName {
    pub name: String,
    pub arguments: Vec<TypeName>,
    pub reference_role: Option<Role>,
    pub span: SourceSpan,
}

/// Structural identity for a nominal type application.
///
/// Source spans, ownership roles, facade paths, and call-site metadata are
/// intentionally excluded from this value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeIdentity {
    name: String,
    arguments: Vec<TypeIdentity>,
}

impl TypeIdentity {
    pub fn from_type_name(type_name: &TypeName) -> Self {
        Self {
            name: type_name.name.clone(),
            arguments: type_name.arguments.iter().map(Self::from_type_name).collect(),
        }
    }

    pub fn key(&self) -> String {
        if self.arguments.is_empty() {
            return self.name.clone();
        }
        format!(
            "{}[{}]",
            self.name,
            self.arguments.iter().map(Self::key).collect::<Vec<_>>().join(",")
        )
    }
}

impl TypeName {
    pub fn canonical_key(&self) -> String {
        self.identity().key()
    }

    pub fn identity(&self) -> TypeIdentity {
        TypeIdentity::from_type_name(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Role, TypeIdentity, TypeName};
    use crate::lexer::SourceSpan;

    fn type_name(name: &str, arguments: Vec<TypeName>, span: usize) -> TypeName {
        TypeName {
            name: name.to_owned(),
            arguments,
            reference_role: None,
            span: SourceSpan::new(span, span + name.len()),
        }
    }

    #[test]
    fn canonical_key_excludes_source_span_and_reference_role() {
        let mut first = type_name("Result", vec![type_name("Int", Vec::new(), 0)], 10);
        let mut second = type_name("Result", vec![type_name("Int", Vec::new(), 200)], 500);
        first.reference_role = Some(Role::Dat);
        second.reference_role = Some(Role::Abs);

        assert_eq!(first.canonical_key(), "Result[Int]");
        assert_eq!(first.canonical_key(), second.canonical_key());
    }

    #[test]
    fn canonical_key_preserves_ordered_generic_arguments() {
        let first = type_name(
            "Pair",
            vec![type_name("Int", Vec::new(), 0), type_name("Bool", Vec::new(), 10)],
            20,
        );
        let second = type_name(
            "Pair",
            vec![type_name("Bool", Vec::new(), 30), type_name("Int", Vec::new(), 40)],
            50,
        );

        assert_eq!(first.canonical_key(), "Pair[Int,Bool]");
        assert_eq!(second.canonical_key(), "Pair[Bool,Int]");
        assert_ne!(first.canonical_key(), second.canonical_key());
    }

    #[test]
    fn identity_is_a_structural_value_object() {
        let type_name = type_name("Result", vec![type_name("Int", Vec::new(), 0)], 10);
        let identity = TypeIdentity::from_type_name(&type_name);

        assert_eq!(identity.key(), "Result[Int]");
        assert_eq!(identity, type_name.identity());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnAccess {
    Owned,
    Abs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReturnType {
    pub access: ReturnAccess,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub kind: GenericParamKind,
    pub bound: Option<TypeName>,
    pub bounds: Vec<TypeName>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenericParamKind {
    Type,
    Const { domain: TypeName },
}
