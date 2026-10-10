use std::collections::BTreeMap;

use crate::ast::{
    BinaryOp, Expr, PrimitiveType, Program, TopLevelDecl, TypeName, UnaryOp, VerbDecl,
};
use crate::lexer::SourceSpan;

use super::{
    BackendBlockId, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp, BackendLayout,
    BackendLayoutProvider, BackendModule, BackendValueId, BackendValueType, SymbolIdentity,
    SymbolKind, lower_verb_with_expressions, lower_verb_with_expressions_as,
};

struct ScalarLayouts;

impl BackendLayoutProvider for ScalarLayouts {
    fn layout_for(&self, _type_name: &TypeName) -> Option<BackendLayout> {
        None
    }
}

pub(crate) fn lower_scalar_program(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
) -> Result<BackendModule, BackendContractError> {
    let verbs = scalar_verbs(program)?;
    let return_types = scalar_return_types(&verbs)?;
    let mut functions = Vec::with_capacity(verbs.len());
    for verb in verbs {
        functions.push(lower_scalar_verb(namespace, verb, entry_name, &return_types)?);
    }
    let module = BackendModule { externals: Vec::new(), functions };
    module.validate()?;
    Ok(module)
}

fn scalar_verbs(program: &Program) -> Result<Vec<&VerbDecl>, BackendContractError> {
    let mut verbs = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => verbs.push(verb),
            TopLevelDecl::ExternalVerb(external) => {
                return Err(unsupported(
                    external.span,
                    "external declarations are not yet part of scalar shared lowering",
                ));
            }
            _ => {}
        }
    }
    Ok(verbs)
}

fn scalar_return_types(
    verbs: &[&VerbDecl],
) -> Result<BTreeMap<String, Option<BackendValueType>>, BackendContractError> {
    verbs
        .iter()
        .map(|verb| {
            let ty = match verb.return_type.as_ref() {
                Some(return_type) => scalar_type(&return_type.ty)?,
                None => None,
            };
            Ok((verb.name.clone(), ty))
        })
        .collect()
}

fn lower_scalar_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
    return_types: &BTreeMap<String, Option<BackendValueType>>,
) -> Result<super::BackendFunction, BackendContractError> {
    let mut provider = ScalarExpressionProvider::new(namespace, verb, return_types);
    match entry_name.filter(|name| *name == verb.name) {
        Some(_) => lower_verb_with_expressions_as(
            SymbolIdentity::entry(&verb.name)
                .map_err(|error| unsupported(verb.span, error.to_string()))?,
            verb,
            &ScalarLayouts,
            &mut provider,
            BackendBlockId(0),
        ),
        None => lower_verb_with_expressions(
            namespace,
            verb,
            &ScalarLayouts,
            &mut provider,
            BackendBlockId(0),
        ),
    }
}

struct ScalarExpressionProvider<'a> {
    namespace: &'a str,
    parameters: BTreeMap<String, BackendValueId>,
    return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    next_value: u32,
}

impl<'a> ScalarExpressionProvider<'a> {
    fn new(
        namespace: &'a str,
        verb: &VerbDecl,
        return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    ) -> Self {
        let parameters = verb
            .params
            .iter()
            .enumerate()
            .map(|(index, parameter)| (parameter.name.clone(), BackendValueId(index as u32)))
            .collect::<BTreeMap<_, _>>();
        Self { namespace, parameters, return_types, next_value: verb.params.len() as u32 }
    }

    fn allocate(&mut self) -> BackendValueId {
        let result = BackendValueId(self.next_value);
        self.next_value += 1;
        result
    }
}

impl BackendExpressionProvider for ScalarExpressionProvider<'_> {
    fn lower_expression(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        match expression {
            Expr::Identifier { name, span } => self
                .parameters
                .get(name)
                .copied()
                .ok_or_else(|| unsupported(*span, format!("unknown scalar binding `{name}`"))),
            Expr::Integer { value, span, .. } => {
                let value = parse_integer(value).ok_or_else(|| {
                    unsupported(*span, format!("invalid scalar integer literal `{value}`"))
                })?;
                let result = self.allocate();
                instructions.push(BackendInstruction::Constant {
                    result,
                    ty: BackendValueType::Integer { signed: true, width: 32 },
                    value,
                    span: *span,
                });
                Ok(result)
            }
            Expr::BoolLiteral { value, span } => {
                let result = self.allocate();
                instructions.push(BackendInstruction::Constant {
                    result,
                    ty: BackendValueType::Bool,
                    value: u64::from(*value),
                    span: *span,
                });
                Ok(result)
            }
            Expr::Grouping { expression, .. } => self.lower_expression(expression, instructions),
            Expr::Unary { .. } => self.lower_unary(expression, instructions),
            Expr::Binary { .. } => self.lower_binary(expression, instructions),
            Expr::Call { .. } => self.lower_call(expression, instructions),
            _ => Err(unsupported(
                expression_span(expression),
                "expression is outside the scalar shared lowering contract",
            )),
        }
    }

    fn callee_for(
        &mut self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        SymbolIdentity::new(self.namespace, SymbolKind::Verb, callee)
            .map_err(|error| unsupported(span, error.to_string()))
    }
}

impl ScalarExpressionProvider<'_> {
    fn lower_unary(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let Expr::Unary { operator, expression, span } = expression else {
            return Err(unsupported(expression_span(expression), "unary expression expected"));
        };
        let operand = self.lower_expression(expression, instructions)?;
        let operation = match operator {
            UnaryOp::Negate => BackendIntegerUnaryOp::Negate,
            UnaryOp::LogicalNot => BackendIntegerUnaryOp::LogicalNot,
            UnaryOp::BitwiseNot => BackendIntegerUnaryOp::BitwiseNot,
        };
        let result = self.allocate();
        instructions.push(BackendInstruction::IntegerUnary {
            result,
            operation,
            operand,
            ty: BackendValueType::Integer { signed: true, width: 32 },
            span: *span,
        });
        Ok(result)
    }

    fn lower_binary(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let Expr::Binary { left, operator, right, span } = expression else {
            return Err(unsupported(expression_span(expression), "binary expression expected"));
        };
        let left = self.lower_expression(left, instructions)?;
        let right = self.lower_expression(right, instructions)?;
        let operation = match operator {
            BinaryOp::Add => BackendIntegerBinaryOp::Add,
            BinaryOp::Subtract => BackendIntegerBinaryOp::Subtract,
            BinaryOp::Multiply => BackendIntegerBinaryOp::Multiply,
            BinaryOp::LessThan
            | BinaryOp::LessEquals
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterEquals
            | BinaryOp::Equals
            | BinaryOp::NotEquals => {
                return self.lower_compare(left, right, *operator, *span, instructions);
            }
            _ => {
                return Err(unsupported(
                    *span,
                    "binary operator is outside the initial scalar lowering contract",
                ));
            }
        };
        let result = self.allocate();
        instructions.push(BackendInstruction::IntegerBinary {
            result,
            operation,
            left,
            right,
            ty: BackendValueType::Integer { signed: true, width: 32 },
            span: *span,
        });
        Ok(result)
    }

    fn lower_compare(
        &mut self,
        left: BackendValueId,
        right: BackendValueId,
        operator: BinaryOp,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let operation = match operator {
            BinaryOp::LessThan => BackendIntegerCompareOp::LessThan,
            BinaryOp::LessEquals => BackendIntegerCompareOp::LessEquals,
            BinaryOp::GreaterThan => BackendIntegerCompareOp::GreaterThan,
            BinaryOp::GreaterEquals => BackendIntegerCompareOp::GreaterEquals,
            BinaryOp::Equals => BackendIntegerCompareOp::Equals,
            BinaryOp::NotEquals => BackendIntegerCompareOp::NotEquals,
            _ => return Err(unsupported(span, "comparison operator expected")),
        };
        let result = self.allocate();
        instructions.push(BackendInstruction::IntegerCompare {
            result,
            operation,
            left,
            right,
            signed: true,
            span,
        });
        Ok(result)
    }

    fn lower_call(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let Expr::Call { callee, arguments, span } = expression else {
            return Err(unsupported(expression_span(expression), "call expression expected"));
        };
        let arguments = arguments
            .iter()
            .map(|argument| self.lower_expression(&argument.expression, instructions))
            .collect::<Result<Vec<_>, _>>()?;
        let result_type = self.return_types.get(callee).copied().flatten();
        let result = result_type.map(|_| self.allocate());
        instructions.push(BackendInstruction::Call {
            callee: self.callee_for(callee, *span)?,
            arguments,
            result,
            span: *span,
        });
        result.ok_or_else(|| unsupported(*span, "unit call cannot produce a value"))
    }
}

fn scalar_type(type_name: &TypeName) -> Result<Option<BackendValueType>, BackendContractError> {
    let ty = match type_name.name.as_str() {
        "Bool" => BackendValueType::Bool,
        "Int" => BackendValueType::Integer { signed: true, width: 32 },
        "Usize" => BackendValueType::Integer { signed: false, width: 64 },
        _ => match crate::ast::primitive_type(&type_name.name) {
            Some(PrimitiveType::Integer { signed, width }) => {
                BackendValueType::Integer { signed, width }
            }
            Some(PrimitiveType::Void) => return Ok(None),
            Some(PrimitiveType::Float { .. }) => {
                return Err(unsupported(
                    type_name.span,
                    "floating-point shared lowering is disabled",
                ));
            }
            None => return Err(unsupported(type_name.span, "non-scalar type in scalar lowering")),
        },
    };
    Ok(Some(ty))
}

fn parse_integer(value: &str) -> Option<u64> {
    value
        .strip_prefix("0x")
        .map(|hex| u64::from_str_radix(hex, 16).ok())
        .unwrap_or_else(|| value.parse().ok())
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Call { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => *span,
    }
}

fn unsupported(span: SourceSpan, reason: impl Into<String>) -> BackendContractError {
    BackendContractError::Unsupported(super::BackendUnsupportedOperation::new(
        "semantic-lowering",
        span,
        super::BackendTargetContext {
            backend: "neutral-contract".to_owned(),
            target: "semantic-model".to_owned(),
        },
        reason,
    ))
}

#[cfg(test)]
mod tests {
    use super::lower_scalar_program;
    use crate::codegen::{
        BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
    };
    use crate::lexer::scan;
    use crate::parser::parse;

    #[test]
    fn lowers_scalar_return_from_parsed_program() {
        let source = "verb main() -> Int { return 42; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("scalar fixture should parse");

        let module = lower_scalar_program("actus_root", &program, None)
            .expect("scalar fixture should lower");
        assert_eq!(module.functions.len(), 1);
        assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(instruction, BackendInstruction::Constant { value: 42, .. })
        ));
    }

    #[test]
    fn lowers_scalar_integer_addition_into_neutral_ir() {
        let source = "verb main() -> Int { return 40 + 2; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("scalar fixture should parse");
        let module = lower_scalar_program("actus_root", &program, None)
            .expect("scalar addition should lower");
        assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(
                instruction,
                BackendInstruction::IntegerBinary { operation: BackendIntegerBinaryOp::Add, .. }
            )
        ));
    }

    #[test]
    fn lowers_scalar_integer_comparison_into_neutral_ir() {
        let source = "verb main() -> Bool { return 1 < 2; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("scalar fixture should parse");
        let module = lower_scalar_program("actus_root", &program, None)
            .expect("scalar comparison should lower");
        assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(
                instruction,
                BackendInstruction::IntegerCompare {
                    operation: BackendIntegerCompareOp::LessThan,
                    signed: true,
                    ..
                }
            )
        ));
    }

    #[test]
    fn lowers_scalar_integer_unary_into_neutral_ir() {
        let source = "verb main() -> Int { return -7; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("scalar fixture should parse");
        let module =
            lower_scalar_program("actus_root", &program, None).expect("scalar unary should lower");
        assert!(module.functions[0].blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(
                instruction,
                BackendInstruction::IntegerUnary { operation: BackendIntegerUnaryOp::Negate, .. }
            )
        ));
    }
}
