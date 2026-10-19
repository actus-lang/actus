use cranelift_codegen::ir::FuncRef;
use cranelift_module::FuncId;

use crate::ast::Program;
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

use super::types::NativeType;

pub use super::super::contract::BackendEmitError;
pub use super::super::symbols::BackendSymbolBindings;

pub(crate) struct FunctionMeta {
    pub(crate) id: FuncId,
    pub(crate) parameter_names: Vec<String>,
    pub(crate) parameter_types: Vec<NativeType>,
    pub(crate) return_type: NativeType,
    pub(crate) dynamic_params: Vec<bool>,
    pub(crate) dynamic_roles: Vec<Option<String>>,
    pub(crate) ins_params: Vec<bool>,
}

pub(crate) struct FunctionRef {
    pub(crate) reference: FuncRef,
    pub(crate) parameter_names: Vec<String>,
    pub(crate) parameter_types: Vec<NativeType>,
    pub(crate) return_type: NativeType,
    pub(crate) dynamic_params: Vec<bool>,
    pub(crate) dynamic_roles: Vec<Option<String>>,
    pub(crate) ins_params: Vec<bool>,
}

pub fn emit_zero_return_object(symbol: &str) -> Result<Vec<u8>, BackendEmitError> {
    let target = TargetSpec::host().map_err(|error| BackendEmitError(error.to_string()))?;
    super::object::emit_i32_object(symbol, 0, &NativeBackendConfiguration::default(), &target)
}

pub fn emit_program_object(program: &Program, symbol: &str) -> Result<Vec<u8>, BackendEmitError> {
    emit_program_object_with_configuration(program, symbol, &NativeBackendConfiguration::default())
}

pub fn emit_program_object_with_configuration(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
) -> Result<Vec<u8>, BackendEmitError> {
    let target = TargetSpec::host().map_err(|error| BackendEmitError(error.to_string()))?;
    emit_program_object_for_target(program, symbol, configuration, &target)
}

pub fn emit_program_object_for_target(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    super::emission::emit_program_object_for_target(
        program,
        super::emission::NativeRootSelection { symbol: Some(symbol), exported: None },
        "actus_root",
        configuration,
        target,
        &BackendSymbolBindings::default(),
        &[],
    )
}

pub fn emit_program_object_for_target_in_namespace(
    program: &Program,
    symbol: &str,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    emit_program_object_for_target_in_namespace_with_bindings(
        program,
        symbol,
        namespace_prefix,
        configuration,
        target,
        &BackendSymbolBindings::default(),
    )
}

pub fn emit_program_object_for_target_in_namespace_with_bindings(
    program: &Program,
    symbol: &str,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
    bindings: &BackendSymbolBindings,
) -> Result<Vec<u8>, BackendEmitError> {
    super::emission::emit_program_object_for_target(
        program,
        super::emission::NativeRootSelection { symbol: Some(symbol), exported: None },
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
) -> Result<Vec<u8>, BackendEmitError> {
    emit_module_object_for_target_in_namespace_with_bindings(
        program,
        namespace_prefix,
        configuration,
        target,
        &BackendSymbolBindings::default(),
    )
}

pub fn emit_module_object_for_target_in_namespace_with_bindings(
    program: &Program,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
    bindings: &BackendSymbolBindings,
) -> Result<Vec<u8>, BackendEmitError> {
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
    bindings: &BackendSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<Vec<u8>, BackendEmitError> {
    emit_module_object_for_target_in_namespace_with_bindings_and_instances_and_roots(
        program,
        namespace_prefix,
        configuration,
        target,
        bindings,
        generic_instances,
        None,
    )
}

pub fn emit_module_object_for_target_in_namespace_with_bindings_and_instances_and_roots(
    program: &Program,
    namespace_prefix: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
    bindings: &BackendSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
    exported_roots: Option<&[String]>,
) -> Result<Vec<u8>, BackendEmitError> {
    super::emission::emit_program_object_for_target(
        program,
        super::emission::NativeRootSelection { symbol: None, exported: exported_roots },
        namespace_prefix,
        configuration,
        target,
        bindings,
        generic_instances,
    )
}

#[cfg(test)]
mod tests {
    use super::BackendSymbolBindings;

    #[test]
    fn keeps_same_source_name_distinct_per_namespace() {
        let bindings = BackendSymbolBindings::new([
            ("actus_mod_left".to_owned(), "read__Int".to_owned(), "left_symbol".to_owned()),
            ("actus_mod_right".to_owned(), "read__Int".to_owned(), "right_symbol".to_owned()),
        ]);

        assert_eq!(bindings.external_symbol("actus_mod_left", "read__Int"), "left_symbol");
        assert_eq!(bindings.external_symbol("actus_mod_right", "read__Int"), "right_symbol");
        assert_eq!(bindings.external_symbol("actus_root", "read__Int"), "read__Int");
    }
}
