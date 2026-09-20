use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    Identifier { name: String, span: SourceSpan },
    Integer { value: String, span: SourceSpan },
    StringLiteral { value: String, span: SourceSpan },
    Borrow { expression: Box<Expr>, span: SourceSpan },
    Call { callee: String, arguments: Vec<Argument>, span: SourceSpan },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Argument {
    pub name: Option<String>,
    pub expression: Expr,
}
