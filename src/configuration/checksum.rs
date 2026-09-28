use std::fs;
use std::path::{Path, PathBuf};

use super::ConfigurationError;

pub(crate) fn content_checksum(root: &Path) -> Result<String, ConfigurationError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hash = 0xcbf29ce484222325_u64;
    for (relative, path) in files {
        update_hash(&mut hash, relative.as_bytes());
        let bytes = fs::read(path).map_err(|error| {
            ConfigurationError(format!("cannot hash dependency content: {error}"))
        })?;
        update_hash(&mut hash, &bytes);
    }
    Ok(format!("{hash:016x}"))
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), ConfigurationError> {
    let entries = fs::read_dir(directory).map_err(|error| {
        ConfigurationError(format!("cannot inspect dependency `{}`: {error}", directory.display()))
    })?;
    for entry in entries {
        let path = entry.map_err(|error| ConfigurationError(error.to_string()))?.path();
        if ignored_dependency_entry(&path) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            files.push(relative_file(root, path));
        }
    }
    Ok(())
}

fn ignored_dependency_entry(path: &Path) -> bool {
    path.file_name().is_some_and(|name| matches!(name.to_str(), Some(".git" | "capsula")))
}

fn relative_file(root: &Path, path: PathBuf) -> (String, PathBuf) {
    let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
    (relative, path)
}

fn update_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}
