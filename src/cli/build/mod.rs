mod artifacts;
#[cfg(feature = "cranelift")]
mod cranelift_shared;
mod emission;
#[cfg(feature = "llvm")]
mod llvm_emission;
mod loading;
#[cfg(feature = "cranelift")]
mod object_manifest;
mod options;
mod shared_emission;

pub(crate) use emission::{EmittedObject, build_file, build_file_quiet, emit_objects};
pub(super) use options::{EmitKind, build_command};
