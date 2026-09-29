use std::collections::HashMap;

use cranelift_codegen::ir::Value;

use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::lowering::LoopTargets;
use super::super::model::NativeCleanupSchedule;
use super::super::native::FunctionRef;
use super::super::types::NativeType;

pub(crate) struct CallLoweringContext<'maps, 'keys> {
    pub(crate) locals: &'maps HashMap<&'keys String, Value>,
    pub(crate) local_types: &'maps HashMap<&'keys String, NativeType>,
    pub(crate) functions: &'maps HashMap<String, FunctionRef>,
    pub(crate) cleanup_schedule: &'maps NativeCleanupSchedule,
    pub(crate) string_data: &'maps StringDataValues,
    pub(crate) layouts: &'maps LayoutRegistry,
    pub(crate) loop_targets: Option<LoopTargets>,
}

impl<'maps, 'keys> CallLoweringContext<'maps, 'keys> {
    pub(crate) fn new(
        locals: &'maps HashMap<&'keys String, Value>,
        local_types: &'maps HashMap<&'keys String, NativeType>,
        functions: &'maps HashMap<String, FunctionRef>,
        cleanup_schedule: &'maps NativeCleanupSchedule,
        string_data: &'maps StringDataValues,
        layouts: &'maps LayoutRegistry,
    ) -> Self {
        Self {
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
            loop_targets: None,
        }
    }

    pub(crate) fn with_loop_targets(mut self, loop_targets: Option<LoopTargets>) -> Self {
        self.loop_targets = loop_targets;
        self
    }
}
