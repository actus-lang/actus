use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    Identifier { name: String, span: SourceSpan },
    Integer { value: String, span: SourceSpan },
    StringLiteral { value: String, span: SourceSpan },
    Binary { left: Box<Expr>, operator: BinaryOp, right: Box<Expr>, span: SourceSpan },
    Borrow { expression: Box<Expr>, span: SourceSpan },
    Call { callee: String, arguments: Vec<Argument>, span: SourceSpan },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Argument {
    pub name: Option<String>,
    pub expression: Expr,
}
