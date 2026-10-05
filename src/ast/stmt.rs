use crate::lexer::SourceSpan;

use super::decl::Role;
use super::expr::Expr;
use super::expr::IfBranch;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForBinding {
    pub role: Role,
    pub name: String,
    pub ty: Option<String>,
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
pub enum Place {
    Binding { name: String, span: SourceSpan },
    Field { object: Box<Place>, field: String, span: SourceSpan },
    Index { target: Box<Place>, index: Expr, span: SourceSpan },
}

impl Place {
    pub fn span(&self) -> SourceSpan {
        match self {
            Self::Binding { span, .. } | Self::Field { span, .. } | Self::Index { span, .. } => {
                *span
            }
        }
    }

    pub fn to_expr(&self) -> Expr {
        match self {
            Self::Binding { name, span } => Expr::Identifier { name: name.clone(), span: *span },
            Self::Field { object, field, span } => Expr::FieldAccess {
                object: Box::new(object.to_expr()),
                field: field.clone(),
                span: *span,
            },
            Self::Index { target, index, span } => Expr::Index {
                target: Box::new(target.to_expr()),
                index: Box::new(index.clone()),
                span: *span,
            },
        }
    }
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
        target: Place,
        value: Expr,
        span: SourceSpan,
    },
    CompoundAssignment {
        target: Place,
        operator: CompoundAssignmentOp,
        value: Expr,
        span: SourceSpan,
    },
    Expression {
        expression: Expr,
        span: SourceSpan,
    },
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<IfBranch>,
        span: SourceSpan,
    },
    Return {
        value: Option<Expr>,
        span: SourceSpan,
    },
    Loop(Block),
    ForRange {
        binding: ForBinding,
        start: Expr,
        end: Expr,
        body: Block,
        span: SourceSpan,
    },
    ForArray {
        binding: ForBinding,
        collection: Expr,
        body: Block,
        span: SourceSpan,
    },
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
