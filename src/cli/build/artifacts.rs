use std::fs;
use std::path::{Path, PathBuf};

use crate::build_graph::write_metadata;
use crate::configuration::{CompilerConfiguration, LinkerFlavor};

use super::options::EmitKind;

pub(super) fn write_artifact(
    input: &str,
    output: &Path,
    object: super::emission::EmittedObject,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> Result<(), String> {
    let persistent_capsula_artifact = output.starts_with(configuration.capsula_target_directory());
    let path =
        object_path(output, emit, persistent_capsula_artifact, configuration.linker_flavor());
    write_object(&path, &object)?;
    if matches!(emit, EmitKind::Executable) {
        let result = crate::codegen::link_objects(&[path.as_path()], output, configuration)
            .map_err(|error| format!("cannot link `{input}`: {error}"));
        if !persistent_capsula_artifact {
            let _ = fs::remove_file(&path);
        }
        result.and_then(|()| write_success_metadata(output, &object, configuration))
    } else {
        write_success_metadata(output, &object, configuration)
    }
}

fn write_success_metadata(
    output: &Path,
    object: &super::emission::EmittedObject,
    configuration: &CompilerConfiguration,
) -> Result<(), String> {
    super::object_manifest::write(&super::object_manifest::path_for(output), object)?;
    write_metadata(output, configuration).map_err(|error| error.to_string())
}

/// The program object (ADR-0113): the output itself for `--emit obj`, and an
/// intermediate object beside the executable for `--emit exe`.
fn object_path(
    output: &Path,
    emit: EmitKind,
    persistent: bool,
    linker_flavor: LinkerFlavor,
) -> PathBuf {
    if matches!(emit, EmitKind::Object) {
        return output.to_owned();
    }
    let extension = if persistent || linker_flavor == LinkerFlavor::Msvc { "obj" } else { "o" };
    output.with_extension(format!("root.{extension}"))
}

fn write_object(path: &Path, object: &super::emission::EmittedObject) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
    }
    fs::write(path, &object.bytes)
        .map_err(|error| format!("cannot write `{}`: {error}", path.display()))
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
