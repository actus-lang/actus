use cranelift_codegen::ir::{Function, Inst};

pub(crate) fn floating_point_instructions(function: &Function) -> Vec<Inst> {
    function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .filter(|&instruction| instruction_uses_float(function, instruction))
        .collect()
}

fn instruction_uses_float(function: &Function, instruction: Inst) -> bool {
    let opcode = format!("{:?}", function.dfg.insts[instruction].opcode());
    opcode.starts_with('F')
        || function
            .dfg
            .inst_args(instruction)
            .iter()
            .any(|value| is_float_type(function.dfg.value_type(*value)))
        || function
            .dfg
            .inst_results(instruction)
            .iter()
            .any(|value| is_float_type(function.dfg.value_type(*value)))
}

fn is_float_type(ty: cranelift_codegen::ir::Type) -> bool {
    ty.is_float() || (ty.is_vector() && ty.lane_type().is_float())
}

#[cfg(test)]
mod tests {
    use cranelift_codegen::ir::InstBuilder;
    use cranelift_codegen::isa::CallConv;
    use cranelift_codegen::isa::TargetFrontendConfig;
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use target_lexicon::PointerWidth;

    use super::floating_point_instructions;

    #[test]
    fn test_zero_float_verification() {
        let mut function = cranelift_codegen::ir::Function::new();
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut function, &mut builder_context);
        let block = builder.create_block();
        builder.switch_to_block(block);
        let left = builder.ins().iconst(cranelift_codegen::ir::types::I32, 1);
        let right = builder.ins().iconst(cranelift_codegen::ir::types::I32, 2);
        let sum = builder.ins().iadd(left, right);
        builder.ins().return_(&[sum]);
        builder.seal_block(block);
        builder.finalize(TargetFrontendConfig {
            default_call_conv: CallConv::Fast,
            pointer_width: PointerWidth::U64,
            page_size_align_log2: 12,
        });

        assert!(floating_point_instructions(&function).is_empty());
    }

    #[test]
    fn detects_float_instructions_in_ir() {
        let mut function = cranelift_codegen::ir::Function::new();
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut function, &mut builder_context);
        let block = builder.create_block();
        builder.switch_to_block(block);
        let left = builder.ins().f32const(1.0);
        let right = builder.ins().f32const(2.0);
        let sum = builder.ins().fadd(left, right);
        builder.ins().return_(&[sum]);
        builder.seal_block(block);
        builder.finalize(TargetFrontendConfig {
            default_call_conv: CallConv::Fast,
            pointer_width: PointerWidth::U64,
            page_size_align_log2: 12,
        });

        assert!(!floating_point_instructions(&function).is_empty());
    }
}
