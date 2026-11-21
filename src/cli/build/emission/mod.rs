mod checks;
#[cfg(feature = "cranelift")]
mod cranelift;
mod linkage;
#[cfg(feature = "llvm")]
mod llvm;
mod objects;
mod shared;

pub(crate) use objects::{
    EmittedObject, build_file, build_file_quiet, build_file_with_mode, emit_object,
};
