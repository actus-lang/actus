use crate::lexer::SourceSpan;

use super::decl::Role;
use super::expr::Expr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stmt {
    OwnerDecl { role: Role, name: String, ty: Option<String>, initializer: Expr, span: SourceSpan },
    Assignment { name: String, value: Expr, span: SourceSpan },
    Expression { expression: Expr, span: SourceSpan },
    Return { value: Option<Expr>, span: SourceSpan },
    Loop(Block),
    Break { span: SourceSpan },
    Continue { span: SourceSpan },
    Drop { name: String, span: SourceSpan },
    Block(Block),
}
