use crate::lexer::SourceSpan;

use super::abi::ForeignAbi;
use super::stmt::Block;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub declarations: Vec<TopLevelDecl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopLevelDecl {
    Verb(VerbDecl),
    ExternalVerb(ExternalVerbDecl),
    Struct(StructDef),
    Enum(EnumDef),
    Role(RoleDecl),
    Perform(PerformDecl),
    OpenSibling(OpenSiblingDecl),
    Import(ImportDecl),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenSiblingDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDecl {
    pub path: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleDecl {
    pub is_open: bool,
    pub name: String,
    pub methods: Vec<RoleMethod>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<ReturnType>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerformDecl {
    pub is_open: bool,
    pub role_name: String,
    pub target: TypeName,
    pub methods: Vec<VerbDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDef {
    pub is_open: bool,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub payload: EnumPayload,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnumPayload {
    Unit,
    Tuple(Vec<TypeName>),
    Struct(Vec<EnumField>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumField {
    pub name: String,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructDef {
    pub is_open: bool,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub fields: Vec<StructField>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructField {
    pub role: StructFieldRole,
    pub name: String,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructFieldRole {
    Value,
    Erg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalVerbDecl {
    pub is_open: bool,
    pub unsafe_boundary: bool,
    pub abi: ForeignAbi,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<ReturnType>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerbDecl {
    pub is_open: bool,
    pub metadata: Vec<MetaAttribute>,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<ReturnType>,
    pub body: Block,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetaAttribute {
    Test,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Param {
    pub role: Role,
    pub name: String,
    pub dispatch: DispatchMode,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchMode {
    Static,
    Dynamic,
}

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
    pub span: SourceSpan,
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
    pub bound: Option<TypeName>,
    pub bounds: Vec<TypeName>,
    pub span: SourceSpan,
}

pub fn builtin_enum_definitions() -> Vec<EnumDef> {
    vec![
        EnumDef {
            is_open: false,
            name: "Option".to_owned(),
            generic_parameters: vec![generic_parameter("T")],
            variants: vec![
                EnumVariant {
                    name: "Some".to_owned(),
                    payload: EnumPayload::Tuple(vec![type_name("T")]),
                    span: zero_span(),
                },
                EnumVariant {
                    name: "None".to_owned(),
                    payload: EnumPayload::Unit,
                    span: zero_span(),
                },
            ],
            span: zero_span(),
        },
        EnumDef {
            is_open: false,
            name: "Result".to_owned(),
            generic_parameters: vec![generic_parameter("T"), generic_parameter("E")],
            variants: vec![
                EnumVariant {
                    name: "Ok".to_owned(),
                    payload: EnumPayload::Tuple(vec![type_name("T")]),
                    span: zero_span(),
                },
                EnumVariant {
                    name: "Err".to_owned(),
                    payload: EnumPayload::Tuple(vec![type_name("E")]),
                    span: zero_span(),
                },
            ],
            span: zero_span(),
        },
    ]
}

fn generic_parameter(name: &str) -> GenericParam {
    GenericParam { name: name.to_owned(), bound: None, bounds: Vec::new(), span: zero_span() }
}

fn type_name(name: &str) -> TypeName {
    TypeName { name: name.to_owned(), arguments: Vec::new(), span: zero_span() }
}

fn zero_span() -> SourceSpan {
    SourceSpan::new(0, 0)
}
