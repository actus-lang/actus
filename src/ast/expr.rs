use crate::lexer::SourceSpan;

use super::pattern::Pattern;
use super::stmt::Block;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    Identifier { name: String, span: SourceSpan },
    Integer { value: String, span: SourceSpan },
    FloatLiteral { value: String, span: SourceSpan },
    StringLiteral { value: String, span: SourceSpan },
    Grouping { expression: Box<Expr>, span: SourceSpan },
    Unary { operator: UnaryOp, expression: Box<Expr>, span: SourceSpan },
    Binary { left: Box<Expr>, operator: BinaryOp, right: Box<Expr>, span: SourceSpan },
    Borrow { expression: Box<Expr>, span: SourceSpan },
    Call { callee: String, arguments: Vec<Argument>, span: SourceSpan },
    MethodCall { receiver: Box<Expr>, method: String, arguments: Vec<Argument>, span: SourceSpan },
    StructLit { name: String, fields: Vec<StructFieldInit>, span: SourceSpan },
    FieldAccess { object: Box<Expr>, field: String, span: SourceSpan },
    Case { mode: CaseMode, subject: Box<Expr>, branches: Vec<CaseBranch>, span: SourceSpan },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseMode {
    Abs,
    Dat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseBranch {
    pub pattern: Pattern,
    pub body: CaseBody,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaseBody {
    Expression(Box<Expr>),
    Block(Block),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Negate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Argument {
    pub name: Option<String>,
    pub expression: Expr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructFieldInit {
    pub name: String,
    pub value: Expr,
    pub span: SourceSpan,
}
