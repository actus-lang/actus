use crate::ast::{BinaryOp, Block, DispatchMode, Expr, IfBranch, Place, Program, Role, Stmt};
use std::collections::BTreeMap;

pub(super) fn supports_scalar_shared_lowering(program: &Program) -> bool {
    let returns_value = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            crate::ast::TopLevelDecl::Verb(verb) => {
                Some((verb.name.clone(), verb.return_type.is_some()))
            }
            crate::ast::TopLevelDecl::ExternalVerb(verb) => {
                Some((verb.name.clone(), verb.return_type.is_some()))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    program.declarations.iter().all(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => {
            verb.generic_parameters.is_empty()
                && verb.params.iter().all(scalar_parameter)
                && verb.return_type.as_ref().is_none_or(|return_type| scalar_type(&return_type.ty))
                && scalar_block(&verb.body, &returns_value, true)
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

fn scalar_block(
    block: &Block,
    returns_value: &BTreeMap<String, bool>,
    allow_assignment: bool,
) -> bool {
    block
        .statements
        .iter()
        .all(|statement| scalar_statement(statement, returns_value, allow_assignment))
}

fn scalar_statement(
    statement: &Stmt,
    returns_value: &BTreeMap<String, bool>,
    allow_assignment: bool,
) -> bool {
    match statement {
        Stmt::OwnerDecl { initializer, .. } => scalar_expression(initializer, returns_value),
        Stmt::Assignment { target, value, .. } if allow_assignment => {
            scalar_place(target) && scalar_expression(value, returns_value)
        }
        Stmt::CompoundAssignment { target, value, .. } if allow_assignment => {
            scalar_place(target) && scalar_expression(value, returns_value)
        }
        Stmt::Assignment { .. } | Stmt::CompoundAssignment { .. } => false,
        Stmt::Expression { expression, .. } => {
            matches!(expression, Expr::Call { .. })
                && scalar_statement_call(expression, returns_value)
        }
        Stmt::If { condition, then_branch, else_branch, .. } => {
            scalar_expression(condition, returns_value)
                && scalar_return_block(then_branch, returns_value)
                && matches!(
                    else_branch,
                    Some(IfBranch::Block(block)) if scalar_return_block(block, returns_value)
                )
        }
        Stmt::Return { value, .. } => {
            value.as_ref().is_none_or(|value| scalar_expression(value, returns_value))
        }
        Stmt::Loop(_) => false,
        Stmt::Block(block) => scalar_block(block, returns_value, true),
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::ForRange { .. } | Stmt::ForArray { .. } | Stmt::Drop { .. } => false,
    }
}

fn scalar_return_block(block: &Block, returns_value: &BTreeMap<String, bool>) -> bool {
    scalar_block(block, returns_value, false)
        && matches!(
            block.statements.last(),
            Some(Stmt::Return { value, .. })
                if value.as_ref().is_none_or(|value| scalar_expression(value, returns_value))
        )
}

fn scalar_place(place: &Place) -> bool {
    matches!(place, Place::Binding { .. })
}

fn scalar_expression(expression: &Expr, returns_value: &BTreeMap<String, bool>) -> bool {
    match expression {
        Expr::Identifier { .. } | Expr::Integer { .. } | Expr::BoolLiteral { .. } => true,
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            scalar_expression(expression, returns_value)
        }
        Expr::Cast { expression, target, .. } => {
            scalar_type(target) && scalar_expression(expression, returns_value)
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
            ) && scalar_expression(left, returns_value)
                && scalar_expression(right, returns_value)
        }
        Expr::Call { callee, arguments, .. } => {
            returns_value.get(callee).copied().unwrap_or(false)
                && arguments
                    .iter()
                    .all(|argument| scalar_expression(&argument.expression, returns_value))
        }
        _ => false,
    }
}

fn scalar_statement_call(expression: &Expr, returns_value: &BTreeMap<String, bool>) -> bool {
    let Expr::Call { callee, arguments, .. } = expression else {
        return false;
    };
    returns_value.contains_key(callee)
        && arguments.iter().all(|argument| scalar_expression(&argument.expression, returns_value))
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
    fn accepts_scalar_assignment_in_the_function_scope() {
        let source = "verb main() -> Int { erg value = 40; value += 2; return value; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let program = parse(tokens).expect("assignment fixture should parse");
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
