use std::fs;
use std::path::{Path, PathBuf};

use crate::build_graph::write_metadata;
use crate::codegen::link_object;
use crate::configuration::CompilerConfiguration;

use super::options::EmitKind;

pub(super) fn write_artifact(
    input: &str,
    output: &Path,
    bytes: Vec<u8>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> Result<(), String> {
    let persistent_capsula_artifact = output.starts_with(configuration.capsula_target_directory());
    let object = matches!(emit, EmitKind::Executable)
        .then(|| output.with_extension(if persistent_capsula_artifact { "obj" } else { "o" }));
    let object_path = object.as_deref().unwrap_or(output);
    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
    }
    fs::write(object_path, bytes)
        .map_err(|error| format!("cannot write `{}`: {error}", object_path.display()))?;
    if let Some(object_path) = object {
        let result = link_object(&object_path, output, configuration)
            .map_err(|error| format!("cannot link `{input}`: {error}"));
        if !persistent_capsula_artifact {
            let _ = fs::remove_file(object_path);
        }
        result
            .and_then(|()| write_metadata(output, configuration).map_err(|error| error.to_string()))
    } else {
        write_metadata(output, configuration).map_err(|error| error.to_string())
    }
}

pub(super) fn default_output(
    input: &str,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> PathBuf {
    let path = Path::new(input);
    let extension = if matches!(emit, EmitKind::Object) { "obj" } else { "bin" };
    configuration
        .capsula_target_directory()
        .join(path.file_stem().unwrap_or_default())
        .with_extension(extension)
}
