use crate::ast::{BinaryOp, Block, DispatchMode, Expr, Program, Role, Stmt};

pub(super) fn supports_scalar_shared_lowering(program: &Program) -> bool {
    program.declarations.iter().all(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => {
            verb.generic_parameters.is_empty()
                && verb.params.iter().all(scalar_parameter)
                && verb.return_type.as_ref().is_none_or(|return_type| scalar_type(&return_type.ty))
                && scalar_block(&verb.body)
        }
        crate::ast::TopLevelDecl::ExternalVerb(verb) => {
            verb.generic_parameters.is_empty()
                && verb.params.iter().all(scalar_parameter)
                && verb.return_type.as_ref().is_none_or(|return_type| scalar_type(&return_type.ty))
        }
        _ => false,
    })
}

fn scalar_parameter(parameter: &crate::ast::Param) -> bool {
    matches!(parameter.role, Role::Erg | Role::Abs)
        && parameter.dispatch == DispatchMode::Static
        && scalar_type(&parameter.ty)
}

fn scalar_type(type_name: &crate::ast::TypeName) -> bool {
    type_name.arguments.is_empty()
        && matches!(
            type_name.name.as_str(),
            "Bool" | "Int" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64"
        )
}

fn scalar_block(block: &Block) -> bool {
    block.statements.iter().all(scalar_statement)
}

fn scalar_statement(statement: &Stmt) -> bool {
    match statement {
        Stmt::OwnerDecl { initializer, .. } => scalar_expression(initializer),
        Stmt::Assignment { .. } | Stmt::CompoundAssignment { .. } => false,
        Stmt::Expression { .. } => false,
        Stmt::If { .. } => false,
        Stmt::Return { value, .. } => value.as_ref().is_none_or(scalar_expression),
        Stmt::Loop(_) => false,
        Stmt::Block(block) => scalar_block(block),
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::ForRange { .. } | Stmt::ForArray { .. } | Stmt::Drop { .. } => false,
    }
}

fn scalar_expression(expression: &Expr) -> bool {
    match expression {
        Expr::Identifier { .. } | Expr::Integer { .. } | Expr::BoolLiteral { .. } => true,
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            scalar_expression(expression)
        }
        Expr::Binary { left, operator, right, .. } => {
            matches!(
                operator,
                BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor
                    | BinaryOp::LessThan
                    | BinaryOp::LessEquals
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterEquals
                    | BinaryOp::Equals
                    | BinaryOp::NotEquals
            ) && scalar_expression(left)
                && scalar_expression(right)
        }
        Expr::Call { .. } => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::supports_scalar_shared_lowering;
    use crate::lexer::scan;
    use crate::parser::parse;

    #[test]
    fn accepts_the_supported_scalar_contract_slice() {
        let source = "verb main() -> Int { return 40 + 2; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let program = parse(tokens).expect("scalar fixture should parse");
        assert!(supports_scalar_shared_lowering(&program));
    }

    #[test]
    fn rejects_constructs_outside_the_scalar_contract_slice() {
        let source = "verb main() -> Int { drop(value); return 0; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let program = parse(tokens).expect("fixture should parse");
        assert!(!supports_scalar_shared_lowering(&program));
    }
}
