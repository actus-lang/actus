use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pattern {
    Variant { enum_name: String, variant: String, payload: VariantPayload, span: SourceSpan },
    Literal { value: LiteralPattern, span: SourceSpan },
    Wildcard { span: SourceSpan },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VariantPayload {
    Unit,
    Positional(Vec<PatternBinding>),
    Named(Vec<NamedPattern>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatternBinding {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedPattern {
    pub name: String,
    pub binding: PatternBinding,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiteralPattern {
    Integer(String),
    Bool(bool),
}
