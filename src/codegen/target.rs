use cranelift_codegen::isa::{self, OwnedTargetIsa};
use cranelift_codegen::settings::{self, Configurable};

use crate::configuration::OptimizationLevel;
use crate::target::{TargetSpec, TargetSpecError};

pub(super) fn build_isa_with_optimization(
    target: &TargetSpec,
    position_independent: bool,
    optimization_level: OptimizationLevel,
) -> Result<OwnedTargetIsa, TargetSpecError> {
    let mut shared_flags = settings::builder();
    shared_flags
        .set("is_pic", &position_independent.to_string())
        .map_err(|error| TargetSpecError::from(error.to_string()))?;
    shared_flags
        .set("opt_level", optimization_level_name(optimization_level))
        .map_err(|error| TargetSpecError::from(error.to_string()))?;
    let shared_flags = settings::Flags::new(shared_flags);
    let isa_builder = isa::lookup(target.triple().clone())
        .map_err(|error| TargetSpecError::from(error.to_string()))?;
    isa_builder.finish(shared_flags).map_err(|error| TargetSpecError::from(error.to_string()))
}

fn optimization_level_name(level: OptimizationLevel) -> &'static str {
    match level {
        OptimizationLevel::None => "none",
        OptimizationLevel::Speed => "speed",
    }
}

#[cfg(test)]
mod tests {
    use cranelift_codegen::settings;
    use target_lexicon::PointerWidth;

    use super::build_isa_with_optimization;
    use crate::configuration::OptimizationLevel;
    use crate::target::TargetSpec;

    #[test]
    fn configures_cranelift_from_the_target_spec() {
        let spec = TargetSpec::host().expect("host target should parse");
        let isa = build_isa_with_optimization(&spec, true, OptimizationLevel::None)
            .expect("Cranelift should accept the target");
        assert_eq!(isa.triple(), spec.triple());
        assert_eq!(isa.pointer_type().bytes(), u32::from(PointerWidth::U64.bytes()));
    }

    #[test]
    fn maps_build_profiles_to_cranelift_optimization_levels() {
        let spec = TargetSpec::host().expect("host target should parse");
        let debug = build_isa_with_optimization(&spec, true, OptimizationLevel::None)
            .expect("debug ISA should build");
        let release = build_isa_with_optimization(&spec, true, OptimizationLevel::Speed)
            .expect("release ISA should build");
        assert_eq!(debug.flags().opt_level(), settings::OptLevel::None);
        assert_eq!(release.flags().opt_level(), settings::OptLevel::Speed);
    }
}
