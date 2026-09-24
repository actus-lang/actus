use std::collections::HashMap;

use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_object::ObjectModule;

use super::function_definition::define_function;
use super::layout::LayoutRegistry;
use super::literals::StringDataIds;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionMeta, NativeEmitError};
use super::performance::{PerformanceDefinition, dispatch_key};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn define_performances(
    module: &mut ObjectModule,
    frontend_config: TargetFrontendConfig,
    definitions: &[PerformanceDefinition<'_>],
    functions: &HashMap<String, FunctionMeta>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataIds,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    for definition in definitions {
        let target_type = NativeType::from_type_name_with_layout(Some(definition.target), layouts);
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
        )?;
    }
    Ok(())
}
