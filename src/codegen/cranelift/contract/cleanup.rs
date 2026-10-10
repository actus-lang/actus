use std::collections::BTreeMap;

use cranelift_codegen::ir::{AbiParam, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::codegen::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupOperand, BackendInstruction,
    BackendModule, BackendValueId, LoweredCleanupInstruction,
};

use super::super::aggregate::Declaration;
use super::super::native::BackendEmitError;

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

    for bridge in bridges {
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
        declarations
            .insert(symbol.to_owned(), Declaration { id, parameter_fragments, return_fragments });
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
    let declaration = declarations.get(bridge_symbol(operand.bridge)).ok_or_else(|| {
        BackendEmitError(format!(
            "Cranelift cleanup bridge `{}` is unavailable",
            bridge_symbol(operand.bridge)
        ))
    })?;
    let reference = module.declare_func_in_func(declaration.id, builder.func);
    let mut arguments = vec![argument];
    if let BackendCleanupBridge::EnumDrop { size } = operand.bridge {
        arguments.push(builder.ins().iconst(module.isa().pointer_type(), i64::from(size)));
    }
    builder.ins().call(reference, &arguments);
    Ok(())
}

fn bridge_symbol(bridge: BackendCleanupBridge) -> &'static str {
    match bridge {
        BackendCleanupBridge::BufferDrop => crate::runtime::BUFFER_DROP_SYMBOL,
        BackendCleanupBridge::RegionDrop => crate::runtime::REGION_DROP_SYMBOL,
        BackendCleanupBridge::EnumDrop { .. } => crate::runtime::ENUM_DROP_SYMBOL,
    }
}
