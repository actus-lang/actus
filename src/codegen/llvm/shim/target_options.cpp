#include "llvm-c/TargetMachine.h"
#include "llvm/Target/TargetMachine.h"

extern "C" void actus_llvm_set_target_options(
    LLVMTargetMachineRef target_machine,
    int emit_stack_size_section,
    int function_sections) {
  auto *machine = reinterpret_cast<llvm::TargetMachine *>(target_machine);
  machine->Options.EmitStackSizeSection = emit_stack_size_section != 0;
  machine->Options.FunctionSections = function_sections != 0;
}
