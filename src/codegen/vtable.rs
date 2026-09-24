use std::collections::HashMap;

use cranelift_module::DataId;
use cranelift_module::{DataDescription, Linkage, Module};
use cranelift_object::ObjectModule;

use super::layout::LayoutRegistry;
use super::native::{FunctionMeta, NativeEmitError};
use super::performance::{PerformanceDefinition, dispatch_key};
use super::types::NativeType;

pub(super) type VtableDataIds = HashMap<String, DataId>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct VtableDefinition {
    pub(super) role_name: String,
    pub(super) target_type: String,
    pub(super) symbol: String,
    pub(super) method_symbols: Vec<String>,
}

pub(super) fn define_vtables(
    module: &mut ObjectModule,
    definitions: &[PerformanceDefinition<'_>],
    functions: &HashMap<String, FunctionMeta>,
    layouts: &LayoutRegistry,
) -> Result<VtableDataIds, NativeEmitError> {
    let mut vtables = group_definitions(definitions, layouts);
    vtables.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    let mut data_ids = HashMap::new();
    for vtable in &vtables {
        let data_id = module
            .declare_data(&vtable.symbol, Linkage::Local, false, false)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        let mut description = DataDescription::new();
        let pointer_bytes = module.isa().pointer_type().bytes() as usize;
        description.define(vec![0; pointer_bytes * vtable.method_symbols.len()].into_boxed_slice());
        for (index, method_symbol) in vtable.method_symbols.iter().enumerate() {
            let function = functions.get(method_symbol).ok_or_else(|| {
                NativeEmitError(format!("missing vtable method `{method_symbol}`"))
            })?;
            let reference = module.declare_func_in_data(function.id, &mut description);
            description.write_function_addr((index * pointer_bytes) as u32, reference);
        }
        module
            .define_data(data_id, &description)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        data_ids.insert(vtable.symbol.clone(), data_id);
    }
    Ok(data_ids)
}

pub(super) fn declare_vtable_values(
    module: &mut ObjectModule,
    function: &mut cranelift_codegen::ir::Function,
    data_ids: &VtableDataIds,
) -> HashMap<String, cranelift_codegen::ir::GlobalValue> {
    data_ids
        .iter()
        .map(|(symbol, data_id)| (symbol.clone(), module.declare_data_in_func(*data_id, function)))
        .collect()
}

fn group_definitions(
    definitions: &[PerformanceDefinition<'_>],
    layouts: &LayoutRegistry,
) -> Vec<VtableDefinition> {
    let mut grouped = HashMap::<(String, String), VtableDefinition>::new();
    for definition in definitions {
        let target = NativeType::from_type_name_with_layout(Some(definition.target), layouts);
        let key = (definition.role_name.clone(), definition.target.name.clone());
        grouped
            .entry(key)
            .or_insert_with(|| VtableDefinition {
                role_name: definition.role_name.clone(),
                target_type: definition.target.name.clone(),
                symbol: vtable_symbol_for_native(&definition.role_name, target),
                method_symbols: Vec::new(),
            })
            .method_symbols
            .push(dispatch_key(target, &definition.method.name));
    }
    grouped.into_values().collect()
}

pub(super) fn vtable_symbol(role: &str, target: &str) -> String {
    format!("actus_vtable_{}_{}", encode(role), encode(target))
}

pub(super) fn vtable_symbol_for_native(role: &str, target: NativeType) -> String {
    let target = match target {
        NativeType::Struct(id) => format!("struct_{id}"),
        NativeType::Enum(id) => format!("enum_{id}"),
        NativeType::Int => "int".to_owned(),
        NativeType::String => "string".to_owned(),
        NativeType::Buffer => "buffer".to_owned(),
        NativeType::FatPointer => "fat_pointer".to_owned(),
    };
    vtable_symbol(role, &target)
}

fn encode(value: &str) -> String {
    value.bytes().fold(String::new(), |mut result, byte| {
        if byte.is_ascii_alphanumeric() {
            result.push(byte as char);
        } else {
            result.push('_');
            result.push_str(&format!("{byte:02x}"));
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::vtable_symbol;

    #[test]
    fn names_vtables_deterministically() {
        assert_eq!(vtable_symbol("Writer", "File"), "actus_vtable_Writer_File");
    }
}
