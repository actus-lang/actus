use cranelift_codegen::ir::FuncRef;
use cranelift_module::FuncId;

use crate::ast::Program;
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

use super::types::NativeType;

mod declarations;
mod emission;
mod object;

#[derive(Debug)]
pub struct NativeEmitError(pub(super) String);

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
    emission::emit_program_object_for_target(program, symbol, configuration, target)
}
