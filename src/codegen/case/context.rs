use std::collections::HashMap;

use cranelift_codegen::ir::Value;

use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::model::NativeCleanupSchedule;
use super::super::native::FunctionRef;
use super::super::types::NativeType;

pub(super) struct CaseLoweringContext<'maps, 'keys> {
    pub(super) locals: &'maps HashMap<&'keys String, Value>,
    pub(super) local_types: &'maps HashMap<&'keys String, NativeType>,
    pub(super) functions: &'maps HashMap<String, FunctionRef>,
    pub(super) cleanup_schedule: &'maps NativeCleanupSchedule,
    pub(super) string_data: &'maps StringDataValues,
    pub(super) layouts: &'maps LayoutRegistry,
}

impl<'maps, 'keys> CaseLoweringContext<'maps, 'keys> {
    pub(super) fn new(
        locals: &'maps HashMap<&'keys String, Value>,
        local_types: &'maps HashMap<&'keys String, NativeType>,
        functions: &'maps HashMap<String, FunctionRef>,
        cleanup_schedule: &'maps NativeCleanupSchedule,
        string_data: &'maps StringDataValues,
        layouts: &'maps LayoutRegistry,
    ) -> Self {
        Self { locals, local_types, functions, cleanup_schedule, string_data, layouts }
    }
}
