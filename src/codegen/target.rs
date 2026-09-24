use cranelift_codegen::isa::{self, OwnedTargetIsa};
use cranelift_codegen::settings::{self, Configurable};

use crate::target::{TargetSpec, TargetSpecError};

pub(super) fn build_isa(
    target: &TargetSpec,
    position_independent: bool,
) -> Result<OwnedTargetIsa, TargetSpecError> {
    let mut shared_flags = settings::builder();
    shared_flags
        .set("is_pic", &position_independent.to_string())
        .map_err(|error| TargetSpecError::from(error.to_string()))?;
    let shared_flags = settings::Flags::new(shared_flags);
    let isa_builder = isa::lookup(target.triple().clone())
        .map_err(|error| TargetSpecError::from(error.to_string()))?;
    isa_builder.finish(shared_flags).map_err(|error| TargetSpecError::from(error.to_string()))
}

#[cfg(test)]
mod tests {
    use target_lexicon::PointerWidth;

    use super::build_isa;
    use crate::target::TargetSpec;

    #[test]
    fn configures_cranelift_from_the_target_spec() {
        let spec = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
        let isa = build_isa(&spec, true).expect("Cranelift should accept the target");
        assert_eq!(isa.triple(), spec.triple());
        assert_eq!(isa.pointer_type().bytes(), u32::from(PointerWidth::U64.bytes()));
    }
}
