use std::collections::HashMap;

use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_object::ObjectModule;

use super::super::function_definition::define_function;
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataIds;
use super::super::model::NativeCleanupSchedule;
use super::super::native::{FunctionMeta, NativeEmitError};
use super::super::types::NativeType;
use super::super::vtable::VtableDataIds;
use super::{PerformanceDefinition, dispatch_key};

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn define_performances(
    module: &mut ObjectModule,
    frontend_config: TargetFrontendConfig,
    definitions: &[PerformanceDefinition<'_>],
    functions: &HashMap<String, FunctionMeta>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataIds,
    layouts: &LayoutRegistry,
    vtable_data: &VtableDataIds,
    namespace_prefix: &str,
) -> Result<(), NativeEmitError> {
    for definition in definitions {
        let target_type = NativeType::from_type_name_with_layout(Some(definition.target), layouts)?;
        let key = dispatch_key(target_type, &definition.method.name);
        let meta = functions.get(&key).ok_or_else(|| {
            NativeEmitError(format!("missing performance function `{}`", definition.symbol))
        })?;
        define_function(
            module,
            frontend_config,
            definition.method,
            meta,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
            vtable_data,
            namespace_prefix,
        )?;
    }
    Ok(())
}
