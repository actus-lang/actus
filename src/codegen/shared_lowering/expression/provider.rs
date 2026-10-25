impl BackendExpressionProvider for SharedExpressionProvider<'_> {
    fn lower_unit_call(
        &mut self,
        callee: &str,
        arguments: &[crate::ast::Argument],
        span: crate::lexer::SourceSpan,
        instructions: &mut Vec<crate::codegen::BackendInstruction>,
    ) -> Result<(), crate::codegen::BackendContractError> {
        let expression = crate::ast::Expr::Call {
            callee: callee.to_owned(),
            arguments: arguments.to_vec(),
            span,
        };
        calls::lower_unit_call(self, &expression, instructions)
    }

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

    fn nominal_type(&self, value: BackendValueId) -> Option<crate::ast::TypeName> {
        self.nominal_type(value).cloned()
    }

    fn remember_nominal_type(
        &mut self,
        value: BackendValueId,
        type_name: crate::ast::TypeName,
    ) {
        self.remember_nominal_type(value, type_name);
    }

    fn copy_value_metadata(&mut self, source: BackendValueId, target: BackendValueId) {
        if let Some(type_name) = self.nominal_type(source).cloned() {
            self.remember_nominal_type(target, type_name);
        }
        let payloads = self
            .enum_payload_types
            .iter()
            .filter_map(|((value, discriminant), ty)| {
                (*value == source).then_some((*discriminant, *ty))
            })
            .collect::<Vec<_>>();
        for (discriminant, ty) in payloads {
            self.remember_enum_payload(target, discriminant, ty);
        }
    }

    fn allocate_value(&mut self, ty: BackendValueType) -> BackendValueId {
        let value = self.allocate();
        self.remember_value(value, ty);
        value
    }

    fn set_binding_value(&mut self, name: &str, value: BackendValueId) {
        state::set_binding_value(self, name, value);
    }

    fn binding_value(&self, name: &str) -> Option<BackendValueId> {
        self.binding_value(name)
    }

    fn append_parameter_writebacks(&self, instructions: &mut Vec<BackendInstruction>) {
        instructions.extend(self.scalar_loan_writebacks());
    }
    fn snapshot_scalar_loan_bindings(&self) -> Vec<(String, BackendValueId)> {
        self.scalar_loan_parameters
            .iter()
            .filter_map(|(name, _, _, _)| {
                self.binding_value(name).map(|value| (name.clone(), value))
            })
            .collect()
    }
    fn restore_scalar_loan_bindings(&mut self, bindings: &[(String, BackendValueId)]) {
        for (name, value) in bindings {
            state::set_binding_value(self, name, *value);
        }
    }
    fn scalar_loan_binding_names(&self) -> Vec<String> {
        self.scalar_loan_parameters.iter().map(|(name, _, _, _)| name.clone()).collect()
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

    fn exit_scope_without_cleanup(&mut self) -> Result<(), BackendContractError> {
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
        let value = if self.is_aggregate_place(value) {
            let BackendValueType::Aggregate { layout } = self
                .value_type(value)
                .ok_or_else(|| unsupported(span, "aggregate owner value has no layout"))?
            else {
                return Err(unsupported(span, "aggregate owner value has an invalid type"));
            };
            let copy = self.allocate_value(BackendValueType::Aggregate { layout });
            if let Some(type_name) = self.nominal_type(value).cloned() {
                self.remember_nominal_type(copy, type_name);
            }
            instructions.push(BackendInstruction::AggregateCopy {
                result: copy,
                source: value,
                layout,
                span,
            });
            copy
        } else {
            value
        };
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
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        let value = self.materialize_aggregate_assignment(value, span, instructions)?;
        if let Some((destination, layout)) = self.aggregate_loan_parameter(name) {
            crate::codegen::shared_lowering::aggregate_assignment::assign_aggregate_loan(
                self,
                destination,
                value,
                layout,
                span,
                instructions,
            )?;
            return Ok(());
        }
        if let Some(previous) = self.binding_value(name).filter(|previous| *previous != value) {
            cleanup::emit_replacement_cleanup(self, name, previous, span, instructions);
        }
        if self.locals.is_empty() {
            return Err(unsupported(span, "scalar assignment has no active lexical scope"));
        }
        if self.locals.last().is_some_and(|scope| scope.contains_key(name)) {
            let scope_index = self.locals.len() - 1;
            self.locals[scope_index].insert(name.to_owned(), value);
            return Ok(());
        }
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
        if self.parameters.contains_key(name) {
            self.parameters.insert(name.to_owned(), value);
            return Ok(());
        }
        Err(unsupported(
            span,
            format!(
                "scalar assignment for `{name}` requires a local owner in the current lexical scope"
            ),
        ))
    }

    fn assign_aggregate_owner(
        &mut self,
        name: &str,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        let Expr::StructLit { fields, .. } = value else {
            return Err(unsupported(span, "aggregate assignment requires a struct literal"));
        };
        let object = Expr::Identifier { name: name.to_owned(), span };
        for field in fields {
            aggregate::lower_field_assignment(
                self,
                &object,
                &field.name,
                &field.value,
                field.span,
                instructions,
            )?;
        }
        Ok(())
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

    fn lower_discarded_expression(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        if let Expr::MethodCall { receiver, method, arguments, span } = expression
            && let Some(result_type) = super::constructors::parse_constructor_receiver_type(
                receiver.as_ref(),
                *span,
            )
            && result_type.name == "Result"
            && result_type.arguments.len() == 2
            && matches!(method.as_str(), "Ok" | "Err")
            && arguments.len() == 1
        {
            let payload = self.lower_expression(&arguments[0].expression, instructions)?;
            let payload_type = self
                .value_type(payload)
                .ok_or_else(|| unsupported(*span, "Result payload type is unavailable"))?;
            if matches!(payload_type, BackendValueType::Bool | BackendValueType::Integer { .. }) {
                return Ok(());
            }
            crate::codegen::shared_lowering::constructors::lower_result_constructor_payload(
                self,
                &result_type,
                method,
                payload,
                payload_type,
                *span,
                instructions,
            )?;
            return Ok(());
        }
        self.lower_expression(expression, instructions).map(|_| ())
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

    fn call_produces_value(&self, callee: &str) -> bool {
        self.return_types.get(callee).is_some_and(Option::is_some)
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

    fn compound_assign_index(
        &mut self,
        target: &Expr,
        index: &Expr,
        operator: crate::ast::CompoundAssignmentOp,
        value: &Expr,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        array_ops::lower_array_element_compound_assignment(
            self,
            target,
            index,
            operator,
            value,
            span,
            instructions,
        )
    }
}
