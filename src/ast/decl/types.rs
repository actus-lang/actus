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
