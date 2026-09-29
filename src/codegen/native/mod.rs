use cranelift_codegen::ir::FuncRef;
use cranelift_module::FuncId;
use std::collections::BTreeMap;

use crate::ast::Program;
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

use super::types::NativeType;

mod declarations;
mod emission;
mod object;

#[derive(Debug)]
pub struct NativeEmitError(pub String);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeSymbolBindings {
    external: BTreeMap<String, String>,
}

impl NativeSymbolBindings {
    pub fn new(bindings: impl IntoIterator<Item = (String, String)>) -> Self {
        Self { external: bindings.into_iter().collect() }
    }

    pub(crate) fn external_symbol<'a>(&'a self, source_name: &'a str) -> &'a str {
        self.external.get(source_name).map(String::as_str).unwrap_or(source_name)
    }
}

pub(super) struct FunctionMeta {
    pub(super) id: FuncId,
    pub(super) parameter_names: Vec<String>,
    pub(super) return_type: NativeType,
    pub(super) dynamic_params: Vec<bool>,
    pub(super) dynamic_roles: Vec<Option<String>>,
    pub(super) ins_params: Vec<bool>,
}

pub(super) struct FunctionRef {
    pub(super) reference: FuncRef,
    pub(super) parameter_names: Vec<String>,
    pub(super) return_type: NativeType,
    pub(super) dynamic_params: Vec<bool>,
    pub(super) dynamic_roles: Vec<Option<String>>,
    pub(super) ins_params: Vec<bool>,
}

impl std::fmt::Display for NativeEmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeEmitError {}

pub fn emit_zero_return_object(symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    let target = TargetSpec::host().map_err(|error| NativeEmitError(error.to_string()))?;
    object::emit_i32_object(symbol, 0, &NativeBackendConfiguration::default(), &target)
}

pub fn emit_program_object(program: &Program, symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    emit_program_object_with_configuration(program, symbol, &NativeBackendConfiguration::default())
}

pub fn emit_program_object_with_configuration(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
) -> Result<Vec<u8>, NativeEmitError> {
    let target = TargetSpec::host().map_err(|error| NativeEmitError(error.to_string()))?;
    emit_program_object_for_target(program, symbol, configuration, &target)
}

pub fn emit_program_object_for_target(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    emission::emit_program_object_for_target(
        program,
        Some(symbol),
        "actus_root",
        configuration,
        target,
        &NativeSymbolBindings::default(),
        &[],
    )
}

pub fn emit_program_object_for_target_in_namespace(
    program: &Program,
    symbol: &str,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    emit_program_object_for_target_in_namespace_with_bindings(
        program,
        symbol,
        namespace_prefix,
        configuration,
        target,
        &NativeSymbolBindings::default(),
    )
}

pub fn emit_program_object_for_target_in_namespace_with_bindings(
    program: &Program,
    symbol: &str,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
    bindings: &NativeSymbolBindings,
) -> Result<Vec<u8>, NativeEmitError> {
    emission::emit_program_object_for_target(
        program,
        Some(symbol),
        namespace_prefix,
        configuration,
        target,
        bindings,
        &[],
    )
}

pub fn emit_module_object_for_target_in_namespace(
    program: &Program,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    emit_module_object_for_target_in_namespace_with_bindings(
        program,
        namespace_prefix,
        configuration,
        target,
        &NativeSymbolBindings::default(),
    )
}

pub fn emit_module_object_for_target_in_namespace_with_bindings(
    program: &Program,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
    bindings: &NativeSymbolBindings,
) -> Result<Vec<u8>, NativeEmitError> {
    emit_module_object_for_target_in_namespace_with_bindings_and_instances(
        program,
        namespace_prefix,
        configuration,
        target,
        bindings,
        &[],
    )
}

pub fn emit_module_object_for_target_in_namespace_with_bindings_and_instances(
    program: &Program,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
    bindings: &NativeSymbolBindings,
    generic_instances: &[super::super::semantic::GenericInstance],
) -> Result<Vec<u8>, NativeEmitError> {
    emission::emit_program_object_for_target(
        program,
        None,
        namespace_prefix,
        configuration,
        target,
        bindings,
        generic_instances,
    )
}
