use std::collections::HashSet;

use crate::semantic::ReachablePerformance;

use crate::ast::{Program, TopLevelDecl, TypeName, VerbDecl};

use super::layout::LayoutRegistry;
use super::native::{FunctionMeta, NativeEmitError};
use super::symbols::{SymbolIdentity, SymbolKind};
use super::types::NativeType;
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

mod emit;

pub(super) use emit::define_performances;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PerformanceImplementation {
    pub(super) role_name: String,
    pub(super) target_type: String,
    pub(super) method_name: String,
    pub(super) symbol: String,
}

#[derive(Default)]
pub(super) struct PerformanceRegistry {
    implementations: Vec<PerformanceImplementation>,
}

pub(super) struct PerformanceDefinition<'a> {
    pub(super) role_name: String,
    pub(super) target: &'a TypeName,
    pub(super) method: &'a VerbDecl,
    pub(super) symbol: String,
}

impl PerformanceRegistry {
    pub(super) fn from_reachable_in_namespace(
        reachable: &[ReachablePerformance],
        namespace_prefix: &str,
    ) -> Self {
        let mut implementations = reachable
            .iter()
            .map(|implementation| PerformanceImplementation {
                role_name: implementation.role_name.clone(),
                target_type: implementation.target_type.clone(),
                method_name: implementation.method_name.clone(),
                symbol: performance_symbol(
                    namespace_prefix,
                    &implementation.role_name,
                    &implementation.target_type,
                    &implementation.method_name,
                ),
            })
            .collect::<Vec<_>>();
        implementations.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        Self { implementations }
    }

    pub(super) fn from_program_in_namespace(
        program: &Program,
        reachable: &[ReachablePerformance],
        namespace_prefix: &str,
    ) -> Self {
        let mut registry = Self::from_reachable_in_namespace(reachable, namespace_prefix);
        for declaration in &program.declarations {
            let TopLevelDecl::Perform(perform) = declaration else { continue };
            let target_type = canonical_type_name(&perform.target);
            for method in &perform.methods {
                registry.implementations.push(PerformanceImplementation {
                    role_name: perform.role_name.clone(),
                    target_type: target_type.clone(),
                    method_name: method.name.clone(),
                    symbol: performance_symbol(
                        namespace_prefix,
                        &perform.role_name,
                        &target_type,
                        &method.name,
                    ),
                });
            }
        }
        registry.implementations.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        registry.implementations.dedup_by(|left, right| left.symbol == right.symbol);
        registry
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        let mut symbols = HashSet::new();
        for implementation in &self.implementations {
            if !symbols.insert(&implementation.symbol) {
                return Err(format!("duplicate performance symbol `{}`", implementation.symbol));
            }
        }
        Ok(())
    }

    pub(super) fn definitions<'a>(
        &self,
        program: &'a Program,
    ) -> Result<Vec<PerformanceDefinition<'a>>, NativeEmitError> {
        self.implementations
            .iter()
            .map(|implementation| find_definition(program, implementation))
            .collect()
    }

    #[cfg(test)]
    fn implementations(&self) -> &[PerformanceImplementation] {
        &self.implementations
    }
}

fn find_definition<'a>(
    program: &'a Program,
    implementation: &PerformanceImplementation,
) -> Result<PerformanceDefinition<'a>, NativeEmitError> {
    let definition = program.declarations.iter().find_map(|declaration| {
        let TopLevelDecl::Perform(perform) = declaration else { return None };
        if perform.role_name != implementation.role_name
            || canonical_type_name(&perform.target) != implementation.target_type
        {
            return None;
        }
        perform.methods.iter().find(|method| method.name == implementation.method_name).map(
            |method| PerformanceDefinition {
                role_name: implementation.role_name.clone(),
                target: &perform.target,
                method,
                symbol: implementation.symbol.clone(),
            },
        )
    });
    definition.ok_or_else(|| {
        NativeEmitError(format!(
            "reachable performance `{}` has no source definition",
            implementation.symbol
        ))
    })
}

pub(super) fn declare_performance_functions(
    module: &mut ObjectModule,
    definitions: &[PerformanceDefinition<'_>],
    layouts: &LayoutRegistry,
) -> Result<std::collections::HashMap<String, FunctionMeta>, NativeEmitError> {
    let mut metadata = std::collections::HashMap::new();
    for definition in definitions {
        let meta = declare_performance_function(module, definition, layouts)?;
        let target_type = NativeType::from_type_name_with_layout(Some(definition.target), layouts)?;
        metadata.insert(dispatch_key(target_type, &definition.method.name), meta);
    }
    Ok(metadata)
}

fn declare_performance_function(
    module: &mut ObjectModule,
    definition: &PerformanceDefinition<'_>,
    layouts: &LayoutRegistry,
) -> Result<FunctionMeta, NativeEmitError> {
    let signature =
        super::declarations::native_signature_for_definition(module, definition.method, layouts)?;
    let id = module
        .declare_function(&definition.symbol, Linkage::Local, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let return_type = NativeType::from_type_name_with_layout(
        definition.method.return_type.as_ref().map(|return_type| &return_type.ty),
        layouts,
    )?;
    Ok(FunctionMeta {
        id,
        parameter_names: definition
            .method
            .params
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect(),
        return_type,
        dynamic_params: dynamic_parameters(definition),
        dynamic_roles: dynamic_roles(definition),
        ins_params: definition
            .method
            .params
            .iter()
            .map(|parameter| {
                parameter.role == crate::ast::Role::Ins
                    && NativeType::try_from_type_name_with_layout(Some(&parameter.ty), layouts)
                        .is_some_and(NativeType::uses_indirect_ins)
            })
            .collect(),
    })
}

fn dynamic_parameters(definition: &PerformanceDefinition<'_>) -> Vec<bool> {
    definition
        .method
        .params
        .iter()
        .map(|parameter| parameter.dispatch == crate::ast::DispatchMode::Dynamic)
        .collect()
}

fn dynamic_roles(definition: &PerformanceDefinition<'_>) -> Vec<Option<String>> {
    definition
        .method
        .params
        .iter()
        .map(|parameter| {
            (parameter.dispatch == crate::ast::DispatchMode::Dynamic)
                .then(|| parameter.ty.name.clone())
        })
        .collect()
}

fn performance_symbol(namespace_prefix: &str, role: &str, target: &str, method: &str) -> String {
    SymbolIdentity::specialized(namespace_prefix, SymbolKind::Performance, method, &[role, target])
        .expect("performance names are validated by semantic analysis")
        .as_str()
        .to_owned()
}

pub(super) fn dispatch_key(target: NativeType, method: &str) -> String {
    format!("actus_dispatch_{}_{}", native_type_key(target), encode_component(method))
}

fn native_type_key(target: NativeType) -> String {
    match target {
        NativeType::Struct(id) => format!("struct_{id}"),
        NativeType::Enum(id) => format!("enum_{id}"),
        NativeType::Int => "int".to_owned(),
        NativeType::Integer { signed, width } => {
            format!("{}int{width}", if signed { "i" } else { "u" })
        }
        NativeType::Float { width } => format!("float{width}"),
        NativeType::Void => "void".to_owned(),
        NativeType::String => "string".to_owned(),
        NativeType::Buffer => "buffer".to_owned(),
        NativeType::FatPointer => "fat_pointer".to_owned(),
        NativeType::Pack(id) => format!("pack_{id}"),
        NativeType::Array(id) => format!("array_{id}"),
        NativeType::Arena(capacity) => format!("arena_{capacity}"),
    }
}

fn canonical_type_name(type_name: &TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

fn encode_component(component: &str) -> String {
    component.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() {
            encoded.push(byte as char);
        } else {
            encoded.push('_');
            encoded.push_str(&format!("{byte:02x}"));
        }
        encoded
    })
}

#[cfg(test)]
mod tests {
    use super::{PerformanceRegistry, performance_symbol};
    use crate::semantic::ReachablePerformance;

    #[test]
    fn creates_stable_symbols_for_simple_performances() {
        assert_eq!(
            performance_symbol("actus_root", "Writer", "File", "write"),
            "actus_root__performance_write__Writer_File"
        );
    }

    #[test]
    fn encodes_type_applications_without_symbol_collisions() {
        let first = performance_symbol("actus_root", "Writer", "Box[Int]", "write");
        let second = performance_symbol("actus_root", "Writer", "Box_BInt", "write");
        assert_ne!(first, second);
    }

    #[test]
    fn sorts_reachable_implementations_by_symbol() {
        let registry = PerformanceRegistry::from_reachable_in_namespace(
            &[
                ReachablePerformance {
                    role_name: "Reader".to_owned(),
                    target_type: "File".to_owned(),
                    method_name: "read".to_owned(),
                },
                ReachablePerformance {
                    role_name: "Writer".to_owned(),
                    target_type: "File".to_owned(),
                    method_name: "write".to_owned(),
                },
            ],
            "actus_root",
        );
        assert_eq!(
            registry.implementations()[0].symbol,
            "actus_root__performance_read__Reader_File"
        );
        assert!(registry.validate().is_ok());
    }
}
