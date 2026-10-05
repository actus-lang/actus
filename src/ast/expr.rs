use crate::lexer::SourceSpan;

use super::decl::TypeName;

use super::pattern::Pattern;
use super::stmt::Block;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    Identifier {
        name: String,
        span: SourceSpan,
    },
    Integer {
        value: String,
        suffix: Option<String>,
        span: SourceSpan,
    },
    BoolLiteral {
        value: bool,
        span: SourceSpan,
    },
    BufferLiteral {
        length: Box<Expr>,
        span: SourceSpan,
    },
    FloatLiteral {
        value: String,
        suffix: Option<String>,
        span: SourceSpan,
    },
    StringLiteral {
        value: String,
        span: SourceSpan,
    },
    Grouping {
        expression: Box<Expr>,
        span: SourceSpan,
    },
    Unary {
        operator: UnaryOp,
        expression: Box<Expr>,
        span: SourceSpan,
    },
    Cast {
        expression: Box<Expr>,
        target: TypeName,
        span: SourceSpan,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
        span: SourceSpan,
    },
    Borrow {
        expression: Box<Expr>,
        span: SourceSpan,
    },
    Try {
        expression: Box<Expr>,
        span: SourceSpan,
    },
    Call {
        callee: String,
        arguments: Vec<Argument>,
        span: SourceSpan,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        arguments: Vec<Argument>,
        span: SourceSpan,
    },
    StructLit {
        name: String,
        type_arguments: Vec<TypeName>,
        fields: Vec<StructFieldInit>,
        span: SourceSpan,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: SourceSpan,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        span: SourceSpan,
    },
    Case {
        mode: CaseMode,
        subject: Box<Expr>,
        branches: Vec<CaseBranch>,
        span: SourceSpan,
    },
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<IfBranch>,
        span: SourceSpan,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IfBranch {
    Block(Block),
    ElseIf(Box<Expr>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseMode {
    Plain,
    Abs,
    Dat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseBranch {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
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
    Remainder,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    LessThan,
    LessEquals,
    GreaterThan,
    GreaterEquals,
    Equals,
    NotEquals,
    LogicalAnd,
    LogicalOr,
}

impl BinaryOp {
    pub const fn is_relational(self) -> bool {
        matches!(self, Self::LessThan | Self::LessEquals | Self::GreaterThan | Self::GreaterEquals)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Negate,
    LogicalNot,
    BitwiseNot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Argument {
    pub name: Option<String>,
    pub role: Option<super::decl::Role>,
    pub role_span: Option<SourceSpan>,
    pub role_resolution: ArgumentRoleResolution,
    pub expression: Expr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgumentRoleResolution {
    Unspecified,
    Explicit,
    Inferred,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructFieldInit {
    pub name: String,
    pub value: Expr,
    pub span: SourceSpan,
}
