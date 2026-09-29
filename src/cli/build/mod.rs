mod artifacts;
mod emission;
mod loading;
mod object_manifest;
mod options;

pub(crate) use emission::{EmittedObject, build_file, build_file_quiet, emit_objects};
pub(super) use options::{EmitKind, build_command};
