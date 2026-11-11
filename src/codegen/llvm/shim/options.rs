use llvm_sys::target_machine::LLVMTargetMachineRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::codegen::llvm) struct TargetMachineOptions {
    pub(in crate::codegen::llvm) emit_stack_size_section: bool,
    pub(in crate::codegen::llvm) function_sections: bool,
}

pub(in crate::codegen::llvm) fn apply(
    target_machine: LLVMTargetMachineRef,
    options: TargetMachineOptions,
) {
    unsafe {
        actus_llvm_set_target_options(
            target_machine,
            i32::from(options.emit_stack_size_section),
            i32::from(options.function_sections),
        );
    }
}

pub(in crate::codegen::llvm) fn disable_x86_call_frame_optimization() {
    unsafe { actus_llvm_disable_x86_call_frame_optimization() }
}

unsafe extern "C" {
    fn actus_llvm_set_target_options(
        target_machine: LLVMTargetMachineRef,
        emit_stack_size_section: i32,
        function_sections: i32,
    );
    fn actus_llvm_disable_x86_call_frame_optimization();
}
