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
mod dispatch;
mod drop;
mod layout;
mod literals;
mod packed;
mod state;
use super::super::{
    BackendCleanupOrigin, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp, BackendSymbolBindings,
    BackendValueId, BackendValueType, SymbolIdentity,
};
use super::support::{expression_span, unsupported};
pub(super) struct SharedExpressionProvider<'a> {
    namespace: &'a str,
    layouts: &'a dyn super::super::BackendLayoutProvider,
    parameters: BTreeMap<String, BackendValueId>,
    owned_parameters: std::collections::BTreeSet<String>,
    locals: Vec<BTreeMap<String, BackendValueId>>,
    verb_name: String,
    return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    return_payload_types: &'a BTreeMap<String, [Option<BackendValueType>; 2]>,
    external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    bindings: Option<&'a BackendSymbolBindings>,
    cleanup_plans: &'a [super::super::BackendCleanupPlan],
    value_types: BTreeMap<BackendValueId, BackendValueType>,
    nominal_types: BTreeMap<BackendValueId, crate::ast::TypeName>,
    enum_payload_types: BTreeMap<(BackendValueId, u32), BackendValueType>,
    next_value: u32,
    loop_depth: usize,
}
impl<'a> SharedExpressionProvider<'a> {
    pub(super) fn new(
        namespace: &'a str,
        verb: &VerbDecl,
        layouts: &'a impl super::super::BackendLayoutProvider,
        return_types: &'a BTreeMap<String, Option<BackendValueType>>,
        return_payload_types: &'a BTreeMap<String, [Option<BackendValueType>; 2]>,
        external_symbols: &'a BTreeMap<String, SymbolIdentity>,
        bindings: Option<&'a BackendSymbolBindings>,
        cleanup_plans: &'a [super::super::BackendCleanupPlan],
    ) -> Self {
        let parameter_bindings = verb
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
        let parameter_types = state::parameter_types(verb, layouts);
        let nominal_types = aggregate::parameter_nominal_types(verb, layouts);
        let enum_payload_types = state::enum_payload_types(verb, layouts);
        Self {
            namespace,
            layouts,
            parameters: parameter_bindings,
            owned_parameters,
            locals: Vec::new(),
            verb_name: verb.name.clone(),
            return_types,
            return_payload_types,
            external_symbols,
            bindings,
            cleanup_plans,
            value_types: parameter_types,
            nominal_types,
            enum_payload_types,
            next_value: verb.params.len() as u32,
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
    fn lower_method_call(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        if arena::is_place_call(self, expression) {
            return arena::lower_place(self, expression, instructions);
        }
        super::constructors::lower_method_call(self, expression, instructions)
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
        self.remember_value(result, BackendValueType::Integer { signed: true, width: 32 });
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
        let result = self.allocate();
        self.remember_value(result, BackendValueType::Bool);
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
}
