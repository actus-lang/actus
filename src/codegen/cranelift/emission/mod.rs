mod declarations;
mod output;
mod verbs;

pub(super) use declarations::{native_signature_for_definition, native_signature_for_entry};
pub(super) use output::{NativeRootSelection, emit_program_object_for_target};
