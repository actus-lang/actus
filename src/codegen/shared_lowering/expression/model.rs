use super::super::super::{
    BackendExpressionProvider, BackendInstruction, BackendPassingMode, BackendSymbolBindings,
    BackendValueId, BackendValueType, SymbolIdentity,
};
use super::parameter_state;
use crate::ast::VerbDecl;
use std::collections::BTreeMap;

pub(crate) struct SharedExpressionProvider<'a> {
    pub(super) namespace: &'a str,
    pub(in crate::codegen::shared_lowering) current_symbol: SymbolIdentity,
    pub(super) layouts: &'a dyn super::super::super::BackendLayoutProvider,
    pub(super) parameters: BTreeMap<String, BackendValueId>,
    pub(super) scalar_loan_parameters:
        Vec<(String, BackendValueId, BackendValueId, BackendValueType)>,
    aggregate_loan_parameters:
        BTreeMap<String, (BackendValueId, super::super::super::BackendLayout)>,
    owned_parameters: std::collections::BTreeSet<String>,
    pub(super) locals: Vec<BTreeMap<String, BackendValueId>>,
    pub(super) verb_name: String,
    pub(super) return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    pub(super) return_nominal_types: &'a BTreeMap<String, Option<crate::ast::TypeName>>,
    pub(super) return_payload_types: &'a BTreeMap<String, [Option<BackendValueType>; 2]>,
    pub(super) parameter_modes: &'a BTreeMap<String, Vec<BackendPassingMode>>,
    pub(super) parameter_types: &'a BTreeMap<String, Vec<BackendValueType>>,
    pub(super) performance_symbols: &'a BTreeMap<(String, String), SymbolIdentity>,
    pub(super) performance_return_types: &'a BTreeMap<(String, String), Option<BackendValueType>>,
    pub(super) performance_return_nominal_types:
        &'a BTreeMap<(String, String), Option<crate::ast::TypeName>>,
    pub(super) external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    pub(super) bindings: Option<&'a BackendSymbolBindings>,
    pub(super) cleanup_plans: &'a [super::super::super::BackendCleanupPlan],
    value_types: BTreeMap<BackendValueId, BackendValueType>,
    nominal_types: BTreeMap<BackendValueId, crate::ast::TypeName>,
    array_big_endian: BTreeMap<BackendValueId, bool>,
    aggregate_places: std::collections::BTreeSet<BackendValueId>,
    heap_owned_aggregate_values: std::collections::BTreeSet<BackendValueId>,
    pub(super) enum_payload_types: BTreeMap<(BackendValueId, u32), BackendValueType>,
    next_value: u32,
    pub(super) loop_depth: usize,
}

impl<'a> SharedExpressionProvider<'a> {
    pub(in crate::codegen::shared_lowering) fn new(
        namespace: &'a str,
        verb: &VerbDecl,
        current_symbol: SymbolIdentity,
        layouts: &'a impl super::super::super::BackendLayoutProvider,
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
        cleanup_plans: &'a [super::super::super::BackendCleanupPlan],
    ) -> Self {
        let parameter_state = parameter_state::initialize(verb, layouts);
        Self {
            namespace,
            current_symbol,
            layouts,
            parameters: parameter_state.parameters,
            scalar_loan_parameters: parameter_state.scalar_loan_parameters,
            aggregate_loan_parameters: parameter_state.aggregate_loan_parameters,
            owned_parameters: parameter_state.owned_parameters,
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
            value_types: parameter_state.value_types,
            nominal_types: parameter_state.nominal_types,
            array_big_endian: BTreeMap::new(),
            aggregate_places: std::collections::BTreeSet::new(),
            heap_owned_aggregate_values: std::collections::BTreeSet::new(),
            enum_payload_types: parameter_state.enum_payload_types,
            next_value: parameter_state.next_value,
            loop_depth: 0,
        }
    }
    pub(in crate::codegen::shared_lowering) fn allocate(&mut self) -> BackendValueId {
        let result = BackendValueId(self.next_value);
        self.next_value += 1;
        result
    }
    pub(in crate::codegen::shared_lowering) fn parameter_value(
        &self,
        name: &str,
    ) -> Option<BackendValueId> {
        self.parameters.get(name).copied()
    }
    pub(in crate::codegen::shared_lowering) fn parameter_is_owned(&self, name: &str) -> bool {
        self.owned_parameters.contains(name)
    }
    pub(in crate::codegen::shared_lowering) fn aggregate_loan_parameter(
        &self,
        name: &str,
    ) -> Option<(BackendValueId, super::super::super::BackendLayout)> {
        self.aggregate_loan_parameters.get(name).copied()
    }
    pub(in crate::codegen::shared_lowering) fn materialize_aggregate_assignment(
        &mut self,
        value: BackendValueId,
        span: crate::lexer::SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, super::super::super::BackendContractError> {
        if !self.is_aggregate_place(value) {
            return Ok(value);
        }
        let BackendValueType::Aggregate { layout } = self.value_type(value).ok_or_else(|| {
            super::super::errors::unsupported(span, "aggregate assignment value has no layout")
        })?
        else {
            return Err(super::super::errors::unsupported(
                span,
                "aggregate assignment value has an invalid type",
            ));
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
        self.remember_heap_owned_aggregate(copy, layout);
        Ok(copy)
    }
    pub(in crate::codegen::shared_lowering) fn scalar_loan_prologue(
        &self,
    ) -> Vec<BackendInstruction> {
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

    pub(in crate::codegen::shared_lowering) fn scalar_loan_writebacks(
        &self,
    ) -> Vec<BackendInstruction> {
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
    pub(in crate::codegen::shared_lowering) fn binding_value(
        &self,
        name: &str,
    ) -> Option<BackendValueId> {
        self.locals
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .or_else(|| self.parameters.get(name).copied())
    }
    pub(in crate::codegen::shared_lowering) fn value_type(
        &self,
        value: BackendValueId,
    ) -> Option<BackendValueType> {
        self.value_types.get(&value).copied()
    }
    pub(in crate::codegen::shared_lowering) fn remember_value(
        &mut self,
        value: BackendValueId,
        ty: BackendValueType,
    ) {
        self.value_types.insert(value, ty);
    }
    pub(in crate::codegen::shared_lowering) fn nominal_type(
        &self,
        value: BackendValueId,
    ) -> Option<&crate::ast::TypeName> {
        self.nominal_types.get(&value)
    }
    pub(in crate::codegen::shared_lowering) fn enum_layout(
        &self,
        type_name: &crate::ast::TypeName,
    ) -> Option<super::super::super::BackendEnumLayout> {
        self.layouts.enum_layout_for(type_name)
    }
    pub(in crate::codegen::shared_lowering) fn enum_variant(
        &self,
        type_name: &crate::ast::TypeName,
        variant: &str,
    ) -> Option<(u32, bool)> {
        self.layouts.enum_variant(type_name, variant)
    }
    pub(in crate::codegen::shared_lowering) fn enum_variant_payload_fields(
        &self,
        type_name: &crate::ast::TypeName,
        variant: &str,
    ) -> Option<Vec<super::super::super::BackendEnumPayloadField>> {
        self.layouts.enum_variant_payload_fields(type_name, variant)
    }

    pub(in crate::codegen::shared_lowering) fn backend_value_type(
        &self,
        type_name: &crate::ast::TypeName,
    ) -> Option<super::super::super::BackendValueType> {
        super::super::scalar_types::backend_value_type(self.layouts, type_name)
    }
    pub(in crate::codegen::shared_lowering) fn enum_variant_for_layout(
        &self,
        layout: super::super::super::BackendEnumLayout,
        variant: &str,
    ) -> Option<(u32, bool)> {
        self.layouts.enum_variant_for_layout(layout, variant)
    }
    pub(in crate::codegen::shared_lowering) fn remember_nominal_type(
        &mut self,
        value: BackendValueId,
        type_name: crate::ast::TypeName,
    ) {
        self.nominal_types.insert(value, type_name);
    }
    pub(in crate::codegen::shared_lowering) fn remember_array_endianness(
        &mut self,
        value: BackendValueId,
        big_endian: bool,
    ) {
        self.array_big_endian.insert(value, big_endian);
    }
    pub(in crate::codegen::shared_lowering) fn array_is_big_endian(
        &self,
        value: BackendValueId,
    ) -> bool {
        self.array_big_endian.get(&value).copied().unwrap_or(false)
    }
    pub(in crate::codegen::shared_lowering) fn remember_aggregate_place(
        &mut self,
        value: BackendValueId,
    ) {
        self.aggregate_places.insert(value);
    }
    pub(in crate::codegen::shared_lowering) fn is_aggregate_place(
        &self,
        value: BackendValueId,
    ) -> bool {
        self.aggregate_places.contains(&value)
    }
    pub(in crate::codegen::shared_lowering) fn remember_heap_owned_aggregate(
        &mut self,
        value: BackendValueId,
        layout: super::super::super::BackendLayout,
    ) {
        if layout.size >= crate::codegen::LARGE_AGGREGATE_THRESHOLD {
            self.heap_owned_aggregate_values.insert(value);
        }
    }
    pub(in crate::codegen::shared_lowering) fn is_heap_owned_aggregate(
        &self,
        value: BackendValueId,
    ) -> bool {
        self.heap_owned_aggregate_values.contains(&value)
    }
    pub(in crate::codegen::shared_lowering) fn remember_enum_payload(
        &mut self,
        value: BackendValueId,
        discriminant: u32,
        ty: BackendValueType,
    ) {
        self.enum_payload_types.insert((value, discriminant), ty);
    }
}
