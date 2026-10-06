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

impl TypeName {
    pub fn canonical_key(&self) -> String {
        if self.arguments.is_empty() {
            self.name.clone()
        } else {
            format!(
                "{}[{}]",
                self.name,
                self.arguments.iter().map(Self::canonical_key).collect::<Vec<_>>().join(",")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Role, TypeName};
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
