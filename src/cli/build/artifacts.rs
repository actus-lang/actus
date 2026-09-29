use std::fs;
use std::path::{Path, PathBuf};

use crate::build_graph::write_metadata;
use crate::configuration::CompilerConfiguration;

use super::options::EmitKind;

pub(super) fn write_artifact(
    input: &str,
    output: &Path,
    objects: Vec<super::emission::EmittedObject>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> Result<(), String> {
    let persistent_capsula_artifact = output.starts_with(configuration.capsula_target_directory());
    let paths = object_paths(output, &objects, emit, persistent_capsula_artifact);
    write_objects(&paths, &objects)?;
    if matches!(emit, EmitKind::Executable) {
        let references = paths.iter().map(PathBuf::as_path).collect::<Vec<_>>();
        let result = crate::codegen::link_objects(&references, output, configuration)
            .map_err(|error| format!("cannot link `{input}`: {error}"));
        if !persistent_capsula_artifact {
            for path in &paths {
                let _ = fs::remove_file(path);
            }
        }
        result
            .and_then(|()| write_metadata(output, configuration).map_err(|error| error.to_string()))
    } else {
        write_metadata(output, configuration).map_err(|error| error.to_string())
    }
}

fn object_paths(
    output: &Path,
    objects: &[super::emission::EmittedObject],
    emit: EmitKind,
    persistent: bool,
) -> Vec<PathBuf> {
    if matches!(emit, EmitKind::Object) {
        let mut paths = vec![output.to_owned()];
        paths.extend(objects.iter().skip(1).map(|object| sidecar_path(output, &object.name)));
        return paths;
    }
    let extension = if persistent { "obj" } else { "o" };
    let root = output.with_extension(format!("root.{extension}"));
    let mut paths = vec![root];
    paths.extend(
        objects
            .iter()
            .skip(1)
            .map(|object| sidecar_path(&output.with_extension(extension), &object.name)),
    );
    paths
}

fn sidecar_path(output: &Path, name: &str) -> PathBuf {
    let stem = output.file_stem().and_then(|value| value.to_str()).unwrap_or("actus");
    let safe_name = name.replace([':', '/', '\\'], "_");
    output.with_file_name(format!(
        "{stem}.module.{safe_name}.{}",
        output.extension().and_then(|value| value.to_str()).unwrap_or("obj")
    ))
}

fn write_objects(
    paths: &[PathBuf],
    objects: &[super::emission::EmittedObject],
) -> Result<(), String> {
    for (path, object) in paths.iter().zip(objects) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
        }
        fs::write(path, &object.bytes)
            .map_err(|error| format!("cannot write `{}`: {error}", path.display()))?;
    }
    Ok(())
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
