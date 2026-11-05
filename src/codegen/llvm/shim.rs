use llvm_sys::target_machine::LLVMTargetMachineRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TargetMachineOptions {
    pub(super) emit_stack_size_section: bool,
    pub(super) function_sections: bool,
}

pub(super) fn apply(target_machine: LLVMTargetMachineRef, options: TargetMachineOptions) {
    unsafe {
        actus_llvm_set_target_options(
            target_machine,
            i32::from(options.emit_stack_size_section),
            i32::from(options.function_sections),
        );
    }
}

unsafe extern "C" {
    fn actus_llvm_set_target_options(
        target_machine: LLVMTargetMachineRef,
        emit_stack_size_section: i32,
        function_sections: i32,
    );
}
