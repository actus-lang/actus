use cranelift_codegen::ir::FuncRef;
use cranelift_module::FuncId;
use std::collections::BTreeMap;

use crate::ast::Program;
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

use super::types::NativeType;

mod declarations;
mod dependencies;
mod emission;
pub(super) mod ir_audit;
mod object;

use emission::NativeRootSelection;

#[derive(Debug)]
pub struct NativeEmitError(pub String);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeSymbolBindings {
    external: BTreeMap<(String, String), String>,
    unique_source: BTreeMap<String, String>,
}

impl NativeSymbolBindings {
    pub fn new(bindings: impl IntoIterator<Item = (String, String, String)>) -> Self {
        let mut external = BTreeMap::new();
        let mut unique_source = BTreeMap::new();
        for (namespace, source, symbol) in bindings {
            external.insert((namespace, source.clone()), symbol.clone());
            match unique_source.get(&source) {
                None => {
                    unique_source.insert(source, symbol);
                }
                Some(existing) if existing == &symbol => {}
                Some(_) => {
                    unique_source.remove(&source);
                }
            }
        }
        Self { external, unique_source }
    }

    pub(crate) fn external_symbol<'a>(&'a self, namespace: &str, source_name: &'a str) -> &'a str {
        self.external
            .get(&(namespace.to_owned(), source_name.to_owned()))
            .map(String::as_str)
            .or_else(|| self.unique_source.get(source_name).map(String::as_str))
            .unwrap_or(source_name)
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

pub(crate) fn reachable_call_names(
    program: &Program,
    symbol: &str,
) -> Result<std::collections::HashSet<String>, NativeEmitError> {
    let verbs = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            crate::ast::TopLevelDecl::Verb(verb) => Some(verb),
            _ => None,
        })
        .collect::<Vec<_>>();
    let external_verbs = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            crate::ast::TopLevelDecl::ExternalVerb(verb) => Some(verb),
            _ => None,
        })
        .collect::<Vec<_>>();
    dependencies::reachable_call_names(&verbs, &external_verbs, Some(symbol))
}

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
        NativeRootSelection { symbol: Some(symbol), exported: None },
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
        NativeRootSelection { symbol: Some(symbol), exported: None },
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
    bindings: &NativeSymbolBindings,
    generic_instances: &[super::super::semantic::GenericInstance],
    exported_roots: Option<&[String]>,
) -> Result<Vec<u8>, NativeEmitError> {
    emission::emit_program_object_for_target(
        program,
        NativeRootSelection { symbol: None, exported: exported_roots },
        namespace_prefix,
        configuration,
        target,
        bindings,
        generic_instances,
    )
}

#[cfg(test)]
mod tests {
    use super::NativeSymbolBindings;

    #[test]
    fn keeps_same_source_name_distinct_per_namespace() {
        let bindings = NativeSymbolBindings::new([
            ("actus_mod_left".to_owned(), "read__Int".to_owned(), "left_symbol".to_owned()),
            ("actus_mod_right".to_owned(), "read__Int".to_owned(), "right_symbol".to_owned()),
        ]);

        assert_eq!(bindings.external_symbol("actus_mod_left", "read__Int"), "left_symbol");
        assert_eq!(bindings.external_symbol("actus_mod_right", "read__Int"), "right_symbol");
        assert_eq!(bindings.external_symbol("actus_root", "read__Int"), "read__Int");
    }
}
