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
        resolved_type: Option<TypeName>,
        generic_arguments: Vec<TypeName>,
        arguments: Vec<Argument>,
        span: SourceSpan,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        arguments: Vec<Argument>,
        resolved_type: Option<TypeName>,
        span: SourceSpan,
    },
    StructLit {
        name: String,
        type_arguments: Vec<TypeName>,
        resolved_type: Option<TypeName>,
        fields: Vec<StructFieldInit>,
        span: SourceSpan,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        resolved_type: Option<TypeName>,
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
    /// `+`: traps when the result does not fit the integer type (ADR-0115).
    Add,
    Subtract,
    Multiply,
    /// `+%`: the result modulo 2^N.
    WrappingAdd,
    WrappingSubtract,
    WrappingMultiply,
    /// `+|`: the result clamped to the type's bounds.
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
    LessThan,
    LessEquals,
    GreaterThan,
    GreaterEquals,
    Equals,
    NotEquals,
    LogicalAnd,
    LogicalOr,
}

impl Expr {
    /// An unsuffixed integer literal, possibly negated or parenthesized. Such
    /// an operand takes the integer type of the other operand of an
    /// arithmetic or bitwise operator (ADR-0119). A cast is not one: it has
    /// the type it names.
    pub fn is_plain_integer_literal(&self) -> bool {
        match self {
            Self::Integer { suffix: None, .. } => true,
            Self::Grouping { expression, .. }
            | Self::Unary { operator: UnaryOp::Negate, expression, .. } => {
                expression.is_plain_integer_literal()
            }
            _ => false,
        }
    }
}

impl BinaryOp {
    /// The wrapping and saturating operators, which take integers only.
    pub const fn is_integer_only_arithmetic(self) -> bool {
        matches!(
            self,
            Self::WrappingAdd
                | Self::WrappingSubtract
                | Self::WrappingMultiply
                | Self::SaturatingAdd
                | Self::SaturatingSubtract
                | Self::SaturatingMultiply
        )
    }

    pub const fn is_relational(self) -> bool {
        matches!(self, Self::LessThan | Self::LessEquals | Self::GreaterThan | Self::GreaterEquals)
    }

    /// The relational operators and `==` / `!=`. Two integer operands of one
    /// of these have one integer type (ADR-0120).
    pub const fn is_comparison(self) -> bool {
        self.is_relational() || matches!(self, Self::Equals | Self::NotEquals)
    }

    /// The arithmetic, wrapping, saturating and bitwise operators, whose two
    /// operands must have one integer type (ADR-0119). Shifts, comparisons
    /// and logical operators are not in this family.
    pub const fn requires_same_operand_type(self) -> bool {
        !matches!(
            self,
            Self::ShiftLeft
                | Self::ShiftRight
                | Self::LessThan
                | Self::LessEquals
                | Self::GreaterThan
                | Self::GreaterEquals
                | Self::Equals
                | Self::NotEquals
                | Self::LogicalAnd
                | Self::LogicalOr
        )
    }

    /// The operator as written in source.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::WrappingAdd => "+%",
            Self::WrappingSubtract => "-%",
            Self::WrappingMultiply => "*%",
            Self::SaturatingAdd => "+|",
            Self::SaturatingSubtract => "-|",
            Self::SaturatingMultiply => "*|",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::BitwiseAnd => "&",
            Self::BitwiseOr => "|",
            Self::BitwiseXor => "^",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
            Self::LessThan => "<",
            Self::LessEquals => "<=",
            Self::GreaterThan => ">",
            Self::GreaterEquals => ">=",
            Self::Equals => "==",
            Self::NotEquals => "!=",
            Self::LogicalAnd => "&&",
            Self::LogicalOr => "||",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    /// `-`: traps when the negation does not fit, such as `-MIN` (ADR-0115).
    Negate,
    /// `-%`: the negation modulo 2^N.
    WrappingNegate,
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
