use std::collections::BTreeMap;

use cranelift_codegen::ir::{AbiParam, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::{BuiltinType, TypeName};
use crate::codegen::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupOperand, BackendCleanupProvider,
    BackendCleanupResolvedDescriptor, BackendContractError, BackendInstruction, BackendLayout,
    BackendModule, BackendValueId, LoweredCleanupInstruction,
};

use super::super::aggregate::Declaration;
use super::super::layout::LayoutRegistry;
use super::super::native::BackendEmitError;
use super::super::types::NativeType;

#[allow(dead_code)]
pub(crate) struct LayoutCleanupProvider<'a> {
    pub(crate) layouts: &'a LayoutRegistry,
}

impl BackendCleanupProvider for LayoutCleanupProvider<'_> {
    fn resolve_binding(
        &self,
        type_name: Option<&TypeName>,
        builtin: Option<BuiltinType>,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        let native_type = if builtin == Some(BuiltinType::Buffer) {
            NativeType::Buffer
        } else if type_name.is_some_and(|name| name.name == "Region") {
            NativeType::Region
        } else {
            return Ok(None);
        };
        let (size, alignment) = self
            .layouts
            .type_layout(native_type)
            .map_err(|_| BackendContractError::InvalidCleanupOperand)?;
        let descriptor = match native_type {
            NativeType::Buffer => BackendCleanupBridgeDescriptor {
                bridge: BackendCleanupBridge::BufferDrop,
                argument: BackendCleanupArgument::Value,
                result: crate::codegen::BackendCleanupResult::Ignored,
            },
            NativeType::Region => BackendCleanupBridgeDescriptor {
                bridge: BackendCleanupBridge::RegionDrop,
                argument: BackendCleanupArgument::Address,
                result: crate::codegen::BackendCleanupResult::Status,
            },
            _ => return Ok(None),
        };
        Ok(Some(BackendCleanupResolvedDescriptor {
            descriptor,
            layout: BackendLayout { size, alignment },
            field: None,
            performance_symbol: None,
        }))
    }

    fn resolve_field(
        &self,
        root_type: &TypeName,
        path: &str,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        let native_type = self
            .layouts
            .native_type_for_type_name(root_type, &mut Vec::new())
            .map_err(|_| BackendContractError::InvalidCleanupOperand)?;
        let NativeType::Struct(root_id) = native_type else {
            return Ok(None);
        };
        let (size, alignment) = self
            .layouts
            .type_layout(native_type)
            .map_err(|_| BackendContractError::InvalidCleanupOperand)?;
        let (field_type, field) = self
            .layouts
            .cleanup_field_layout_by_id(root_id, &root_type.name, path)
            .map_err(|_| BackendContractError::InvalidCleanupOperand)?;
        let descriptor = match field_type {
            NativeType::Buffer => BackendCleanupBridgeDescriptor {
                bridge: BackendCleanupBridge::BufferDrop,
                argument: BackendCleanupArgument::FieldAddress,
                result: crate::codegen::BackendCleanupResult::Ignored,
            },
            NativeType::Region => BackendCleanupBridgeDescriptor {
                bridge: BackendCleanupBridge::RegionDrop,
                argument: BackendCleanupArgument::FieldAddress,
                result: crate::codegen::BackendCleanupResult::Status,
            },
            _ => return Ok(None),
        };
        Ok(Some(BackendCleanupResolvedDescriptor {
            descriptor,
            layout: BackendLayout { size, alignment },
            field: Some(BackendCleanupField { ..field }),
            performance_symbol: None,
        }))
    }
}

pub(super) fn declare_cleanup_bridges(
    module: &mut ObjectModule,
    backend_module: &BackendModule,
    declarations: &mut BTreeMap<String, Declaration>,
) -> Result<(), BackendEmitError> {
    let mut bridges = Vec::new();
    for function in &backend_module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let BackendInstruction::Cleanup { operand: Some(operand), .. } = instruction else {
                    continue;
                };
                if !bridges.contains(&operand.bridge) {
                    bridges.push(operand.bridge);
                }
            }
        }
    }
    if backend_module.externals.iter().any(|external| external.runtime_pointer_return)
        || bridges.iter().any(|bridge| matches!(bridge, BackendCleanupBridge::RegionDrop))
    {
        let enum_drop = BackendCleanupBridge::EnumDrop { size: 0 };
        if !bridges.contains(&enum_drop) {
            bridges.push(enum_drop);
        }
    }

    let needs_aggregate_allocator = backend_module.functions.iter().any(|function| {
        function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(instruction, BackendInstruction::AggregateConstruct { layout, .. }
                    if layout.size >= crate::codegen::LARGE_AGGREGATE_THRESHOLD)
            })
        })
    });
    if needs_aggregate_allocator {
        let pointer_type = module.isa().pointer_type();
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(pointer_type));
        signature.params.push(AbiParam::new(pointer_type));
        signature.returns.push(AbiParam::new(pointer_type));
        let id = module
            .declare_function(
                crate::runtime::AGGREGATE_ALLOCATE_SYMBOL,
                Linkage::Import,
                &signature,
            )
            .map_err(|error| BackendEmitError(error.to_string()))?;
        declarations.insert(
            crate::runtime::AGGREGATE_ALLOCATE_SYMBOL.to_owned(),
            Declaration {
                id,
                parameter_fragments: vec![1, 1],
                parameter_types: vec![Vec::new(), Vec::new()],
                return_fragments: 1,
                return_slot: None,
                runtime_pointer_return: false,
                niche_pointer_return: false,
                runtime_result_layout: None,
            },
        );
    }

    for bridge in bridges {
        if matches!(bridge, BackendCleanupBridge::PerformanceDrop) {
            continue;
        }
        let symbol = bridge_symbol(bridge);
        if declarations.contains_key(symbol) {
            continue;
        }
        let pointer_type = module.isa().pointer_type();
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(pointer_type));
        let parameter_fragments = if matches!(bridge, BackendCleanupBridge::EnumDrop { .. }) {
            signature.params.push(AbiParam::new(pointer_type));
            vec![1, 1]
        } else if matches!(bridge, BackendCleanupBridge::AggregateDrop { .. }) {
            signature.params.push(AbiParam::new(pointer_type));
            signature.params.push(AbiParam::new(pointer_type));
            vec![1, 1, 1]
        } else {
            vec![1]
        };
        let return_fragments = if matches!(bridge, BackendCleanupBridge::RegionDrop) {
            signature.returns.push(AbiParam::new(cranelift_codegen::ir::types::I32));
            1
        } else {
            0
        };
        let id = module
            .declare_function(symbol, Linkage::Import, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        let parameter_count = parameter_fragments.len();
        declarations.insert(
            symbol.to_owned(),
            Declaration {
                id,
                parameter_fragments,
                parameter_types: vec![Vec::new(); parameter_count],
                return_fragments,
                return_slot: None,
                runtime_pointer_return: false,
                niche_pointer_return: false,
                runtime_result_layout: None,
            },
        );
    }
    Ok(())
}

pub(super) fn lower_cleanup(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    action: &LoweredCleanupInstruction,
    operand: Option<BackendCleanupOperand>,
    declarations: &BTreeMap<String, Declaration>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    if matches!(action, LoweredCleanupInstruction::EndBorrow { .. }) {
        return Ok(());
    }
    let operand = operand
        .ok_or_else(|| BackendEmitError("Cranelift cleanup operand is missing".to_owned()))?;
    let value = values.get(&operand.value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift cleanup value {} is unavailable", operand.value.0))
    })?;
    let argument = match operand.argument {
        BackendCleanupArgument::Value | BackendCleanupArgument::Address => value,
        BackendCleanupArgument::FieldAddress => {
            let field = operand.field.ok_or_else(|| {
                BackendEmitError("Cranelift field cleanup requires a field sublayout".to_owned())
            })?;
            builder.ins().iadd_imm_s(value, i64::from(field.offset))
        }
    };
    let symbol = operand.performance_symbol.as_ref().map_or_else(
        || bridge_symbol(operand.bridge).to_owned(),
        |symbol| symbol.as_str().to_owned(),
    );
    let declaration = declarations.get(&symbol).ok_or_else(|| {
        BackendEmitError(format!("Cranelift cleanup bridge `{}` is unavailable", symbol))
    })?;
    let reference = module.declare_func_in_func(declaration.id, builder.func);
    let mut arguments = vec![argument];
    if let BackendCleanupBridge::EnumDrop { size } = operand.bridge {
        arguments.push(builder.ins().iconst(module.isa().pointer_type(), i64::from(size)));
    } else if let BackendCleanupBridge::AggregateDrop { size, alignment } = operand.bridge {
        arguments.push(builder.ins().iconst(module.isa().pointer_type(), i64::from(size)));
        arguments.push(builder.ins().iconst(module.isa().pointer_type(), i64::from(alignment)));
    }
    builder.ins().call(reference, &arguments);
    Ok(())
}

fn bridge_symbol(bridge: BackendCleanupBridge) -> &'static str {
    match bridge {
        BackendCleanupBridge::BufferDrop => crate::runtime::BUFFER_DROP_SYMBOL,
        BackendCleanupBridge::RegionDrop => crate::runtime::REGION_DROP_SYMBOL,
        BackendCleanupBridge::EnumDrop { .. } => crate::runtime::ENUM_DROP_SYMBOL,
        BackendCleanupBridge::AggregateDrop { .. } => crate::runtime::AGGREGATE_DROP_SYMBOL,
        BackendCleanupBridge::PerformanceDrop => "",
    }
}

#[cfg(test)]
mod tests {
    use super::LayoutCleanupProvider;
    use crate::codegen::BackendCleanupProvider;
    use crate::codegen::cranelift::layout::LayoutRegistry;
    use crate::lexer::scan;
    use crate::parser::parse;

    #[test]
    fn resolves_nested_runtime_field_through_native_layout_registry() {
        let (tokens, errors) = scan(
            "struct Inner { erg first: Buffer, erg second: Region, } struct Outer { erg inner: Inner, } verb main() { }",
        );
        assert!(errors.is_empty());
        let program = parse(tokens).expect("cleanup provider source should parse");
        let layouts = LayoutRegistry::from_program(&program, cranelift_codegen::ir::types::I64)
            .expect("cleanup provider layout should pass");
        let provider = LayoutCleanupProvider { layouts: &layouts };
        let root = crate::ast::TypeName {
            name: "Outer".to_owned(),
            arguments: Vec::new(),
            reference_role: None,
            span: crate::lexer::SourceSpan::new(0, 5),
        };
        let first =
            provider.resolve_field(&root, "inner.first").expect("buffer field should resolve");
        assert_eq!(first.expect("runtime field").field.expect("field metadata").offset, 0);
        let second =
            provider.resolve_field(&root, "inner.second").expect("region field should resolve");
        assert_eq!(second.expect("runtime field").field.expect("field metadata").offset, 8);
        assert!(provider.resolve_field(&root, "inner.missing").is_err());
    }
}
