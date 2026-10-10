use std::collections::BTreeMap;

use crate::ast::{
    BinaryOp, Expr, ExternalVerbDecl, Program, TopLevelDecl, TypeName, UnaryOp, VerbDecl,
};
use crate::lexer::SourceSpan;

mod support;

use support::{expression_span, parse_integer, required_scalar_type, scalar_type, unsupported};

use super::{
    BackendBlockId, BackendContractError, BackendExpressionProvider, BackendExternal,
    BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
    BackendLayout, BackendLayoutProvider, BackendModule, BackendParameter, BackendPassingMode,
    BackendReturn, BackendReturnMode, BackendSymbolBindings, BackendValueId, BackendValueType,
    SymbolIdentity, SymbolKind, lower_verb_with_expressions, lower_verb_with_expressions_as,
};

struct ScalarLayouts;

impl BackendLayoutProvider for ScalarLayouts {
    fn layout_for(&self, _type_name: &TypeName) -> Option<BackendLayout> {
        None
    }
}

#[doc(hidden)]
pub fn lower_scalar_program(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
) -> Result<BackendModule, BackendContractError> {
    lower_program_to_backend_module(namespace, program, entry_name, None)
}

pub(crate) fn lower_program_to_backend_module(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BackendModule, BackendContractError> {
    let (verbs, externals) = scalar_declarations(program)?;
    let return_types = scalar_return_types(&verbs, &externals)?;
    let external_symbols = scalar_external_symbols(namespace, &externals, bindings)?;
    let mut functions = Vec::with_capacity(verbs.len());
    for verb in verbs {
        functions.push(lower_scalar_verb(
            namespace,
            verb,
            entry_name,
            &return_types,
            &external_symbols,
        )?);
    }
    let module = BackendModule {
        externals: externals
            .iter()
            .map(|external| scalar_external(external, &external_symbols))
            .collect::<Result<_, _>>()?,
        functions,
    };
    module.validate()?;
    Ok(module)
}

fn scalar_declarations(
    program: &Program,
) -> Result<(Vec<&VerbDecl>, Vec<&ExternalVerbDecl>), BackendContractError> {
    let mut verbs = Vec::new();
    let mut externals = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => verbs.push(verb),
            TopLevelDecl::ExternalVerb(external) => externals.push(external),
            _ => {}
        }
    }
    Ok((verbs, externals))
}

fn scalar_external_symbols(
    namespace: &str,
    externals: &[&ExternalVerbDecl],
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BTreeMap<String, SymbolIdentity>, BackendContractError> {
    externals
        .iter()
        .map(|external| {
            let source = external.native_symbol.as_deref().unwrap_or(&external.name);
            let symbol = bindings
                .map(|bindings| bindings.external_symbol(namespace, source))
                .unwrap_or(source);
            Ok((
                external.name.clone(),
                SymbolIdentity::external_raw(symbol)
                    .map_err(|error| unsupported(external.span, error.to_string()))?,
            ))
        })
        .collect()
}

fn scalar_external(
    external: &ExternalVerbDecl,
    symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<BackendExternal, BackendContractError> {
    let parameters = external
        .params
        .iter()
        .map(|parameter| {
            Ok(BackendParameter {
                name: parameter.name.clone(),
                role: parameter.role.clone(),
                ty: required_scalar_type(&parameter.ty)?,
                passing: BackendPassingMode::Direct,
            })
        })
        .collect::<Result<Vec<_>, BackendContractError>>()?;
    let return_value = match external.return_type.as_ref() {
        Some(return_type) => BackendReturn {
            role: None,
            ty: required_scalar_type(&return_type.ty)?,
            passing: BackendReturnMode::Direct,
        },
        None => BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
    };
    let symbol = symbols.get(&external.name).cloned().ok_or_else(|| {
        unsupported(external.span, format!("external symbol `{}` is unavailable", external.name))
    })?;
    Ok(BackendExternal { symbol, parameters, return_value })
}

fn scalar_return_types(
    verbs: &[&VerbDecl],
    externals: &[&ExternalVerbDecl],
) -> Result<BTreeMap<String, Option<BackendValueType>>, BackendContractError> {
    let verb_types = verbs
        .iter()
        .map(|verb| {
            let ty = match verb.return_type.as_ref() {
                Some(return_type) => scalar_type(&return_type.ty)?,
                None => None,
            };
            Ok((verb.name.clone(), ty))
        })
        .collect::<Result<BTreeMap<_, _>, BackendContractError>>()?;
    let external_types = externals
        .iter()
        .map(|external| {
            let ty = match external.return_type.as_ref() {
                Some(return_type) => scalar_type(&return_type.ty)?,
                None => None,
            };
            Ok((external.name.clone(), ty))
        })
        .collect::<Result<BTreeMap<_, _>, BackendContractError>>()?;
    Ok(verb_types.into_iter().chain(external_types).collect())
}

fn lower_scalar_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
    return_types: &BTreeMap<String, Option<BackendValueType>>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<super::BackendFunction, BackendContractError> {
    let mut provider =
        ScalarExpressionProvider::new(namespace, verb, return_types, external_symbols);
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
    locals: Vec<BTreeMap<String, BackendValueId>>,
    return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    next_value: u32,
}

impl<'a> ScalarExpressionProvider<'a> {
    fn new(
        namespace: &'a str,
        verb: &VerbDecl,
        return_types: &'a BTreeMap<String, Option<BackendValueType>>,
        external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    ) -> Self {
        let parameters = verb
            .params
            .iter()
            .enumerate()
            .map(|(index, parameter)| (parameter.name.clone(), BackendValueId(index as u32)))
            .collect::<BTreeMap<_, _>>();
        Self {
            namespace,
            parameters,
            locals: Vec::new(),
            return_types,
            external_symbols,
            next_value: verb.params.len() as u32,
        }
    }

    fn allocate(&mut self) -> BackendValueId {
        let result = BackendValueId(self.next_value);
        self.next_value += 1;
        result
    }
}

impl BackendExpressionProvider for ScalarExpressionProvider<'_> {
    fn enter_scope(&mut self) {
        self.locals.push(BTreeMap::new());
    }

    fn exit_scope(&mut self) {
        self.locals.pop();
    }

    fn bind_owner(
        &mut self,
        name: &str,
        value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        let Some(scope) = self.locals.last_mut() else {
            return Err(unsupported(span, "scalar owner declaration has no active lexical scope"));
        };
        scope.insert(name.to_owned(), value);
        Ok(())
    }

    fn assign_owner(
        &mut self,
        name: &str,
        value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        let Some(scope) = self.locals.last_mut() else {
            return Err(unsupported(span, "scalar assignment has no active lexical scope"));
        };
        if scope.contains_key(name) {
            scope.insert(name.to_owned(), value);
            return Ok(());
        }
        if self.locals.len() == 1 && self.parameters.contains_key(name) {
            self.parameters.insert(name.to_owned(), value);
            return Ok(());
        }
        Err(unsupported(
            span,
            "scalar assignment requires a local owner in the current lexical scope",
        ))
    }

    fn lower_integer_binary(
        &mut self,
        operation: BackendIntegerBinaryOp,
        left: BackendValueId,
        right: BackendValueId,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        if matches!(operation, BackendIntegerBinaryOp::Divide | BackendIntegerBinaryOp::Remainder) {
            instructions.push(BackendInstruction::TrapIfZero {
                value: right,
                reason: "integer division or remainder by zero".to_owned(),
                span,
            });
        }
        if matches!(operation, BackendIntegerBinaryOp::Divide) {
            instructions.push(BackendInstruction::TrapIfSignedDivisionOverflow {
                left,
                right,
                width: 32,
                reason: "signed integer division overflow".to_owned(),
                span,
            });
        }
        if matches!(
            operation,
            BackendIntegerBinaryOp::ShiftLeft | BackendIntegerBinaryOp::ShiftRight
        ) {
            instructions.push(BackendInstruction::TrapIfShiftCountOutOfRange {
                count: right,
                width: 32,
                reason: "integer shift count exceeds operand width".to_owned(),
                span,
            });
        }
        let result = self.allocate();
        instructions.push(BackendInstruction::IntegerBinary {
            result,
            operation,
            left,
            right,
            ty: BackendValueType::Integer { signed: true, width: 32 },
            span,
        });
        Ok(result)
    }

    fn lower_expression(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        match expression {
            Expr::Identifier { name, span } => self
                .locals
                .iter()
                .rev()
                .find_map(|scope| scope.get(name).copied())
                .or_else(|| self.parameters.get(name).copied())
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
        self.external_symbols
            .get(callee)
            .cloned()
            .map_or_else(|| SymbolIdentity::new(self.namespace, SymbolKind::Verb, callee), Ok)
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
            BinaryOp::Divide => BackendIntegerBinaryOp::Divide,
            BinaryOp::Remainder => BackendIntegerBinaryOp::Remainder,
            BinaryOp::ShiftLeft => BackendIntegerBinaryOp::ShiftLeft,
            BinaryOp::ShiftRight => BackendIntegerBinaryOp::ShiftRight,
            BinaryOp::BitwiseAnd => BackendIntegerBinaryOp::BitwiseAnd,
            BinaryOp::BitwiseOr => BackendIntegerBinaryOp::BitwiseOr,
            BinaryOp::BitwiseXor => BackendIntegerBinaryOp::BitwiseXor,
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
        self.lower_integer_binary(operation, left, right, *span, instructions)
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
