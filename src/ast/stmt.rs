use crate::lexer::SourceSpan;

use super::decl::Role;
use super::expr::Expr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompoundAssignmentOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompoundAssignmentTarget {
    Identifier(String),
    Field { object: Expr, field: String },
    Index { target: Expr, index: Expr },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stmt {
    OwnerDecl {
        role: Role,
        name: String,
        ty: Option<String>,
        initializer: Expr,
        span: SourceSpan,
    },
    Assignment {
        name: String,
        value: Expr,
        span: SourceSpan,
    },
    FieldAssignment {
        object: Expr,
        field: String,
        value: Expr,
        span: SourceSpan,
    },
    IndexAssignment {
        target: Expr,
        index: Expr,
        value: Expr,
        span: SourceSpan,
    },
    CompoundAssignment {
        target: CompoundAssignmentTarget,
        operator: CompoundAssignmentOp,
        value: Expr,
        span: SourceSpan,
    },
    Expression {
        expression: Expr,
        span: SourceSpan,
    },
    Return {
        value: Option<Expr>,
        span: SourceSpan,
    },
    Loop(Block),
    Break {
        span: SourceSpan,
    },
    Continue {
        span: SourceSpan,
    },
    Drop {
        name: String,
        span: SourceSpan,
    },
    Block(Block),
}
