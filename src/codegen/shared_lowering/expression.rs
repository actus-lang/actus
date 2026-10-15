// The provider constructor stores all explicit neutral lowering contracts.
#![allow(clippy::too_many_arguments)]

use crate::ast::{BinaryOp, CaseMode, Expr, Role, UnaryOp, VariantPayload, VerbDecl};
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;
use std::collections::BTreeMap;
mod aggregate;
mod arena;
mod arithmetic;
mod array_ops;
mod buffer;
mod calls;
mod case;
mod cast;
mod cleanup;
pub(crate) use cleanup::descriptor_applies;
mod dispatch;
mod drop;
mod layout;
mod literals;
mod packed;
mod state;
use super::super::{
    BackendCleanupOrigin, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp, BackendPassingMode,
    BackendSymbolBindings, BackendValueId, BackendValueType, SymbolIdentity,
};
use super::support::{expression_span, unsupported};
pub(super) struct SharedExpressionProvider<'a> {
    namespace: &'a str,
    layouts: &'a dyn super::super::BackendLayoutProvider,
    parameters: BTreeMap<String, BackendValueId>,
    scalar_loan_parameters: Vec<(String, BackendValueId, BackendValueId, BackendValueType)>,
    owned_parameters: std::collections::BTreeSet<String>,
    locals: Vec<BTreeMap<String, BackendValueId>>,
    verb_name: String,
    return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    return_nominal_types: &'a BTreeMap<String, Option<crate::ast::TypeName>>,
    return_payload_types: &'a BTreeMap<String, [Option<BackendValueType>; 2]>,
    parameter_modes: &'a BTreeMap<String, Vec<BackendPassingMode>>,
    parameter_types: &'a BTreeMap<String, Vec<BackendValueType>>,
    performance_symbols: &'a BTreeMap<(String, String), SymbolIdentity>,
    performance_return_types: &'a BTreeMap<(String, String), Option<BackendValueType>>,
    performance_return_nominal_types: &'a BTreeMap<(String, String), Option<crate::ast::TypeName>>,
    external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    bindings: Option<&'a BackendSymbolBindings>,
    cleanup_plans: &'a [super::super::BackendCleanupPlan],
    value_types: BTreeMap<BackendValueId, BackendValueType>,
    nominal_types: BTreeMap<BackendValueId, crate::ast::TypeName>,
    array_big_endian: BTreeMap<BackendValueId, bool>,
    enum_payload_types: BTreeMap<(BackendValueId, u32), BackendValueType>,
    next_value: u32,
    loop_depth: usize,
}

pub(super) fn lower_unit_call(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    calls::lower_unit_call(provider, expression, instructions)
}
impl<'a> SharedExpressionProvider<'a> {
    pub(super) fn new(
        namespace: &'a str,
        verb: &VerbDecl,
        layouts: &'a impl super::super::BackendLayoutProvider,
        return_types: &'a BTreeMap<String, Option<BackendValueType>>,
        return_nominal_types: &'a BTreeMap<String, Option<crate::ast::TypeName>>,
        return_payload_types: &'a BTreeMap<String, [Option<BackendValueType>; 2]>,
        parameter_modes: &'a BTreeMap<String, Vec<BackendPassingMode>>,
        call_parameter_types: &'a BTreeMap<String, Vec<BackendValueType>>,
        performance_symbols: &'a BTreeMap<(String, String), SymbolIdentity>,
        performance_return_types: &'a BTreeMap<(String, String), Option<BackendValueType>>,
        performance_return_nominal_types: &'a BTreeMap<
            (String, String),
            Option<crate::ast::TypeName>,
        >,
        external_symbols: &'a BTreeMap<String, SymbolIdentity>,
        bindings: Option<&'a BackendSymbolBindings>,
        cleanup_plans: &'a [super::super::BackendCleanupPlan],
    ) -> Self {
        let mut parameter_bindings = verb
            .params
            .iter()
            .enumerate()
            .map(|(index, parameter)| (parameter.name.clone(), BackendValueId(index as u32)))
            .collect::<BTreeMap<_, _>>();
        let owned_parameters = verb
            .params
            .iter()
            .filter(|parameter| parameter.role == Role::Erg)
            .map(|parameter| parameter.name.clone())
            .collect();
        let mut parameter_types = state::parameter_types(verb, layouts);
        let nominal_types = aggregate::parameter_nominal_types(verb, layouts);
        let enum_payload_types = state::enum_payload_types(verb, layouts);
        let mut scalar_loan_parameters = Vec::new();
        let mut next_value = verb.params.len() as u32;
        for (index, parameter) in verb.params.iter().enumerate() {
            let Some(BackendValueType::Bool | BackendValueType::Integer { .. }) =
                parameter_types.get(&BackendValueId(index as u32)).copied()
            else {
                continue;
            };
            if !matches!(
                crate::codegen::lower_parameter(parameter, layouts)
                    .ok()
                    .map(|parameter| parameter.passing),
                Some(BackendPassingMode::ExclusiveScalarLoan { .. })
            ) {
                continue;
            }
            let address = BackendValueId(index as u32);
            let loaded = BackendValueId(next_value);
            next_value += 1;
            let ty = parameter_types[&address];
            parameter_bindings.insert(parameter.name.clone(), loaded);
            parameter_types.insert(loaded, ty);
            scalar_loan_parameters.push((parameter.name.clone(), address, loaded, ty));
        }
        Self {
            namespace,
            layouts,
            parameters: parameter_bindings,
            scalar_loan_parameters,
            owned_parameters,
            locals: Vec::new(),
            verb_name: verb.name.clone(),
            return_types,
            return_nominal_types,
            return_payload_types,
            parameter_modes,
            parameter_types: call_parameter_types,
            performance_symbols,
            performance_return_types,
            performance_return_nominal_types,
            external_symbols,
            bindings,
            cleanup_plans,
            value_types: parameter_types,
            nominal_types,
            array_big_endian: BTreeMap::new(),
            enum_payload_types,
            next_value,
            loop_depth: 0,
        }
    }
    pub(super) fn allocate(&mut self) -> BackendValueId {
        let result = BackendValueId(self.next_value);
        self.next_value += 1;
        result
    }
    pub(super) fn parameter_value(&self, name: &str) -> Option<BackendValueId> {
        self.parameters.get(name).copied()
    }
    pub(super) fn parameter_is_owned(&self, name: &str) -> bool {
        self.owned_parameters.contains(name)
    }
    pub(super) fn scalar_loan_prologue(&self) -> Vec<BackendInstruction> {
        self.scalar_loan_parameters
            .iter()
            .map(|(_, address, loaded, ty)| BackendInstruction::ScalarLoanLoad {
                result: *loaded,
                address: *address,
                ty: *ty,
                span: crate::lexer::SourceSpan::new(0, 0),
            })
            .collect()
    }

    pub(super) fn scalar_loan_writebacks(&self) -> Vec<BackendInstruction> {
        self.scalar_loan_parameters
            .iter()
            .filter_map(|(name, address, _, ty)| {
                self.binding_value(name).map(|value| BackendInstruction::ScalarLoanStore {
                    address: *address,
                    value,
                    ty: *ty,
                    span: crate::lexer::SourceSpan::new(0, 0),
                })
            })
            .collect()
    }
    pub(super) fn binding_value(&self, name: &str) -> Option<BackendValueId> {
        self.locals
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .or_else(|| self.parameters.get(name).copied())
    }
    pub(super) fn value_type(&self, value: BackendValueId) -> Option<BackendValueType> {
        self.value_types.get(&value).copied()
    }
    pub(super) fn remember_value(&mut self, value: BackendValueId, ty: BackendValueType) {
        self.value_types.insert(value, ty);
    }
    pub(super) fn nominal_type(&self, value: BackendValueId) -> Option<&crate::ast::TypeName> {
        self.nominal_types.get(&value)
    }
    pub(super) fn enum_layout(
        &self,
        type_name: &crate::ast::TypeName,
    ) -> Option<super::super::BackendEnumLayout> {
        self.layouts.enum_layout_for(type_name)
    }
    pub(super) fn enum_variant(
        &self,
        type_name: &crate::ast::TypeName,
        variant: &str,
    ) -> Option<(u32, bool)> {
        self.layouts.enum_variant(type_name, variant)
    }
    pub(super) fn enum_variant_payload_type(
        &self,
        type_name: &crate::ast::TypeName,
        variant: &str,
    ) -> Option<BackendValueType> {
        self.layouts.enum_variant_payload_type(type_name, variant)
    }
    pub(super) fn enum_variant_for_layout(
        &self,
        layout: super::super::BackendEnumLayout,
        variant: &str,
    ) -> Option<(u32, bool)> {
        self.layouts.enum_variant_for_layout(layout, variant)
    }
    pub(super) fn remember_nominal_type(
        &mut self,
        value: BackendValueId,
        type_name: crate::ast::TypeName,
    ) {
        self.nominal_types.insert(value, type_name);
    }
    pub(super) fn remember_array_endianness(&mut self, value: BackendValueId, big_endian: bool) {
        self.array_big_endian.insert(value, big_endian);
    }
    pub(super) fn array_is_big_endian(&self, value: BackendValueId) -> bool {
        self.array_big_endian.get(&value).copied().unwrap_or(false)
    }
    pub(super) fn remember_enum_payload(
        &mut self,
        value: BackendValueId,
        discriminant: u32,
        ty: BackendValueType,
    ) {
        self.enum_payload_types.insert((value, discriminant), ty);
    }
}
include!("expression/provider.rs");

impl SharedExpressionProvider<'_> {
    pub(super) fn call_returns_unit(&self, callee: &str) -> bool {
        self.return_types.get(callee).is_some_and(Option::is_none)
    }

    fn lower_method_call(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        if arena::is_place_call(self, expression) {
            return arena::lower_place(self, expression, instructions);
        }
        let Expr::MethodCall { receiver, method, arguments, span } = expression else {
            return Err(unsupported(expression_span(expression), "method call expected"));
        };
        let receiver_is_binding = self.is_binding_receiver(receiver);
        if !receiver_is_binding {
            return super::constructors::lower_method_call(self, expression, instructions);
        }
        let receiver_value = dispatch::lower_expression(self, receiver, instructions)?;
        let receiver_type = self
            .nominal_type(receiver_value)
            .ok_or_else(|| unsupported(*span, "performance receiver has no nominal type"))?;
        let key = (receiver_type.canonical_key(), method.clone());
        let callee = self.method_callee(&key, method, *span)?;
        let mut lowered_arguments = Vec::with_capacity(arguments.len() + 1);
        lowered_arguments.push(receiver_value);
        lowered_arguments.extend(
            arguments
                .iter()
                .map(|argument| {
                    dispatch::lower_expression(self, &argument.expression, instructions)
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        let result_type = self.method_result_type(&key, method);
        let result = result_type.map(|_| self.allocate());
        if let (Some(result), Some(result_type)) = (result, result_type) {
            self.remember_call_result(result, result_type, &key, method);
        }
        instructions.push(BackendInstruction::Call {
            callee,
            arguments: lowered_arguments,
            result,
            span: *span,
        });
        result.ok_or_else(|| unsupported(*span, "unit performance method cannot produce a value"))
    }

    fn method_callee(
        &self,
        key: &(String, String),
        method: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        self.performance_symbols
            .get(key)
            .cloned()
            .or_else(|| {
                self.return_types
                    .contains_key(method)
                    .then(|| calls::callee_for(self, method, span).ok())
                    .flatten()
            })
            .ok_or_else(|| unsupported(span, "method has no static symbol"))
    }

    fn is_binding_receiver(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Identifier { name, .. } => self.binding_value(name).is_some(),
            Expr::FieldAccess { object, .. } | Expr::Index { target: object, .. } => {
                self.is_binding_receiver(object)
            }
            _ => false,
        }
    }

    fn method_result_type(&self, key: &(String, String), method: &str) -> Option<BackendValueType> {
        self.performance_return_types
            .get(key)
            .copied()
            .flatten()
            .or_else(|| self.return_types.get(method).copied().flatten())
    }

    fn remember_call_result(
        &mut self,
        result: BackendValueId,
        result_type: BackendValueType,
        key: &(String, String),
        method: &str,
    ) {
        self.remember_value(result, result_type);
        let nominal_type = self
            .performance_return_nominal_types
            .get(key)
            .and_then(Option::as_ref)
            .or_else(|| self.return_nominal_types.get(method).and_then(Option::as_ref));
        if let Some(nominal_type) = nominal_type {
            self.remember_nominal_type(result, nominal_type.clone());
        }
        if let Some(payload_types) = self.return_payload_types.get(method) {
            for (discriminant, payload_type) in payload_types.iter().enumerate() {
                if let Some(payload_type) = payload_type {
                    self.remember_enum_payload(result, discriminant as u32, *payload_type);
                }
            }
        }
    }

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
        let result_type = if matches!(operator, UnaryOp::LogicalNot) {
            BackendValueType::Bool
        } else {
            BackendValueType::Integer { signed: true, width: 32 }
        };
        self.remember_value(result, result_type);
        instructions.push(BackendInstruction::IntegerUnary {
            result,
            operation,
            operand,
            ty: result_type,
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
            BinaryOp::LogicalAnd => BackendIntegerBinaryOp::LogicalAnd,
            BinaryOp::LogicalOr => BackendIntegerBinaryOp::LogicalOr,
            BinaryOp::LessThan
            | BinaryOp::LessEquals
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterEquals
            | BinaryOp::Equals
            | BinaryOp::NotEquals => {
                return self.lower_compare(left, right, *operator, *span, instructions);
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
        let left_type = self
            .value_type(left)
            .ok_or_else(|| unsupported(span, "left comparison operand type is unavailable"))?;
        let right_type = self
            .value_type(right)
            .ok_or_else(|| unsupported(span, "right comparison operand type is unavailable"))?;
        if let (
            BackendValueType::Float { width: left_width },
            BackendValueType::Float { width: right_width },
        ) = (left_type, right_type)
        {
            if left_width != right_width {
                return Err(unsupported(span, "floating-point comparison widths must match"));
            }
            let result = self.allocate();
            self.remember_value(result, BackendValueType::Bool);
            instructions.push(BackendInstruction::FloatCompare {
                result,
                operation,
                left,
                right,
                ty: left_type,
                span,
            });
            return Ok(result);
        }
        let result = self.allocate();
        self.remember_value(result, BackendValueType::Bool);
        instructions.push(BackendInstruction::IntegerCompare {
            result,
            operation,
            left,
            right,
            signed: matches!(left_type, BackendValueType::Integer { signed: true, .. }),
            span,
        });
        Ok(result)
    }
}
