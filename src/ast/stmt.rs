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
    WrappingAdd,
    WrappingSubtract,
    WrappingMultiply,
    SaturatingAdd,
    SaturatingSubtract,
    SaturatingMultiply,
    Divide,
    Remainder,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
}

impl CompoundAssignmentOp {
    /// The binary operator `target op= value` applies.
    pub const fn binary_operator(self) -> crate::ast::BinaryOp {
        use crate::ast::BinaryOp;
        match self {
            Self::Add => BinaryOp::Add,
            Self::Subtract => BinaryOp::Subtract,
            Self::Multiply => BinaryOp::Multiply,
            Self::WrappingAdd => BinaryOp::WrappingAdd,
            Self::WrappingSubtract => BinaryOp::WrappingSubtract,
            Self::WrappingMultiply => BinaryOp::WrappingMultiply,
            Self::SaturatingAdd => BinaryOp::SaturatingAdd,
            Self::SaturatingSubtract => BinaryOp::SaturatingSubtract,
            Self::SaturatingMultiply => BinaryOp::SaturatingMultiply,
            Self::Divide => BinaryOp::Divide,
            Self::Remainder => BinaryOp::Remainder,
            Self::BitwiseAnd => BinaryOp::BitwiseAnd,
            Self::BitwiseOr => BinaryOp::BitwiseOr,
            Self::BitwiseXor => BinaryOp::BitwiseXor,
            Self::ShiftLeft => BinaryOp::ShiftLeft,
            Self::ShiftRight => BinaryOp::ShiftRight,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Place {
    Binding { name: String, span: SourceSpan },
    Field { object: Box<Place>, field: String, span: SourceSpan },
    Index { target: Box<Place>, index: Box<Expr>, span: SourceSpan },
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
                resolved_type: None,
                span: *span,
            },
            Self::Index { target, index, span } => Expr::Index {
                target: Box::new(target.to_expr()),
                index: index.clone(),
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
        resolved_type: Option<crate::ast::TypeName>,
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
        end: Box<Expr>,
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
