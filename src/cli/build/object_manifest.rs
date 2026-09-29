use std::fs;
use std::path::Path;

use serde::Serialize;

use super::emission::EmittedObject;

const OBJECT_MANIFEST_VERSION: u32 = 1;

#[derive(Serialize)]
struct ObjectManifest {
    format_version: u32,
    objects: Vec<ObjectManifestEntry>,
}

#[derive(Serialize)]
struct ObjectManifestEntry {
    name: String,
    symbols: Vec<String>,
}

pub(super) fn write(path: &Path, objects: &[EmittedObject]) -> Result<(), String> {
    let manifest = ObjectManifest {
        format_version: OBJECT_MANIFEST_VERSION,
        objects: objects
            .iter()
            .map(|object| ObjectManifestEntry {
                name: object.name.clone(),
                symbols: object.symbols.iter().cloned().collect(),
            })
            .collect(),
    };
    let source = toml::to_string(&manifest)
        .map_err(|error| format!("cannot encode `{}`: {error}", path.display()))?;
    fs::write(path, source).map_err(|error| format!("cannot write `{}`: {error}", path.display()))
}

pub(super) fn path_for(artifact: &Path) -> std::path::PathBuf {
    artifact.with_extension("symbols")
}
