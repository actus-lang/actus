use crate::ast::{BinaryOp, CaseMode, Expr, UnaryOp, VariantPayload, VerbDecl};
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;
use std::collections::BTreeMap;
mod aggregate;
mod arithmetic;
mod calls;
mod case;
mod cast;
mod cleanup;
mod dispatch;
mod drop;
mod layout;
mod literals;
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
        let parameter_types = state::parameter_types(verb, layouts);
        let nominal_types = aggregate::parameter_nominal_types(verb, layouts);
        let enum_payload_types = state::enum_payload_types(verb, layouts);
        Self {
            namespace,
            layouts,
            parameters: parameter_bindings,
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
impl BackendExpressionProvider for SharedExpressionProvider<'_> {
    fn supports_range_loops(&self) -> bool {
        true
    }
    fn supports_array_loops(&self) -> bool {
        true
    }
    fn array_capacity(
        &self,
        value: BackendValueId,
        span: SourceSpan,
    ) -> Result<u32, BackendContractError> {
        let Some(type_name) = self.nominal_type(value) else {
            return Err(unsupported(span, "array loop collection has no nominal type"));
        };
        if type_name.name != "Array" || type_name.arguments.len() != 2 {
            return Err(unsupported(span, "array loop requires a fixed array collection"));
        }
        type_name.arguments[1]
            .name
            .parse::<u32>()
            .map_err(|_| unsupported(span, "array capacity is not a concrete integer"))
    }

    fn enter_loop(&mut self) {
        self.loop_depth += 1;
    }

    fn exit_loop(&mut self) {
        self.loop_depth = self.loop_depth.saturating_sub(1);
    }
    fn value_type(&self, value: BackendValueId) -> Option<BackendValueType> {
        self.value_type(value)
    }

    fn allocate_value(&mut self, ty: BackendValueType) -> BackendValueId {
        let value = self.allocate();
        self.remember_value(value, ty);
        value
    }

    fn set_binding_value(&mut self, name: &str, value: BackendValueId) {
        state::set_binding_value(self, name, value);
    }
    fn enter_scope(&mut self, _span: SourceSpan) {
        self.locals.push(BTreeMap::new());
    }
    fn exit_scope(
        &mut self,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        let skip_scope_cleanup = matches!(
            instructions.last(),
            Some(BackendInstruction::Jump { span, .. }) if self.is_loop_exit(*span)
        );
        if !skip_scope_cleanup {
            cleanup::emit_scope_cleanup(self, span, instructions);
        }
        self.locals.pop();
        Ok(())
    }

    fn distribute_scope_cleanup(
        &mut self,
        span: SourceSpan,
        blocks: &mut Vec<super::super::BackendBlock>,
        start: usize,
    ) -> Result<(), BackendContractError> {
        for block in blocks.iter_mut().skip(start) {
            if matches!(
                block.instructions.last(),
                Some(BackendInstruction::Return { .. })
                    | Some(BackendInstruction::Fail { .. })
                    | Some(BackendInstruction::Jump { .. })
            ) {
                cleanup::emit_scope_cleanup(self, span, &mut block.instructions);
            }
        }
        Ok(())
    }

    fn is_loop_exit(&self, span: SourceSpan) -> bool {
        self.cleanup_plans.iter().any(|plan| {
            matches!(
                plan.origin,
                BackendCleanupOrigin::Loop { span: boundary, .. } if boundary == span
            )
        })
    }

    fn emit_loop_cleanup(
        &mut self,
        span: SourceSpan,
        kind: LoopExitKind,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        cleanup::append_loop_cleanup(self, span, kind, instructions);
        Ok(())
    }

    fn drop_binding(
        &mut self,
        name: &str,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        drop::lower_binding(self, name, span, instructions)
    }
    fn bind_owner(
        &mut self,
        name: &str,
        value: BackendValueId,
        declared_type: Option<&str>,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        let value = state::coerce_owner_value(self, value, declared_type, span, instructions)?;
        if let Some(declared_type) =
            declared_type.and_then(|name| crate::semantic::parse_type_name_key(name, span))
            && self.layouts.layout_for(&declared_type).is_some()
        {
            self.remember_nominal_type(value, declared_type);
        }
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
        if self.locals.is_empty() {
            return Err(unsupported(span, "scalar assignment has no active lexical scope"));
        }
        if self.locals.last().is_some_and(|scope| scope.contains_key(name)) {
            let scope_index = self.locals.len() - 1;
            self.locals[scope_index].insert(name.to_owned(), value);
            return Ok(());
        }
        if self.loop_depth > 0 {
            if let Some(scope_index) = self
                .locals
                .iter()
                .enumerate()
                .rev()
                .skip(1)
                .find_map(|(index, scope)| scope.contains_key(name).then_some(index))
            {
                self.locals[scope_index].insert(name.to_owned(), value);
                return Ok(());
            }
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
        self.lower_integer_binary_impl(operation, left, right, span, instructions)
    }

    fn lower_expression(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        dispatch::lower_expression(self, expression, instructions)
    }

    fn coerce_return_value(
        &mut self,
        value: BackendValueId,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let return_type = self.return_types.get(&self.verb_name).copied().flatten();
        state::coerce_value_to_type(self, value, return_type, span, instructions)
    }

    fn lower_enum_variant_condition_value(
        &mut self,
        subject: BackendValueId,
        variant: &str,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        case::lower_enum_variant_condition_value(self, subject, variant, span, instructions)
    }

    fn bind_enum_variant_payload(
        &mut self,
        subject: BackendValueId,
        variant: &str,
        payload: Option<&VariantPayload>,
        mode: CaseMode,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        case::bind_enum_variant_payload(self, subject, variant, payload, mode, span, instructions)
    }

    fn callee_for(
        &mut self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        calls::callee_for(self, callee, span)
    }

    fn lower_array_element(
        &mut self,
        collection: &Expr,
        index: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        aggregate::lower_array_element(self, collection, index, span, instructions)
    }

    fn assign_field(
        &mut self,
        object: &Expr,
        field: &str,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        aggregate::lower_field_assignment(self, object, field, value, span, instructions)
    }

    fn assign_index(
        &mut self,
        target: &Expr,
        index: &Expr,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        aggregate::lower_array_element_assignment(self, target, index, value, span, instructions)
    }

    fn compound_assign_field(
        &mut self,
        object: &Expr,
        field: &str,
        operator: crate::ast::CompoundAssignmentOp,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        aggregate::lower_field_compound_assignment(
            self,
            object,
            field,
            operator,
            value,
            span,
            instructions,
        )
    }
}

impl SharedExpressionProvider<'_> {
    fn lower_method_call(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
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
