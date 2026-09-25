use std::fs;
use std::path::{Path, PathBuf};

const BLOCK_SIZE: usize = 512;
const FILE_MODE: u32 = 0o644;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageArchive {
    pub path: PathBuf,
    pub checksum: String,
    pub files: Vec<String>,
}

#[derive(Debug)]
pub struct ArchiveError(pub String);

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ArchiveError {}

pub fn create_package_archive(root: &Path, output: &Path) -> Result<PackageArchive, ArchiveError> {
    let mut files = Vec::new();
    collect_files(root, root, output, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut archive = Vec::new();
    for (relative, path) in &files {
        append_file(&mut archive, relative, path)?;
    }
    archive.resize(archive.len() + BLOCK_SIZE * 2, 0);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| ArchiveError(format!("cannot create archive directory: {error}")))?;
    }
    fs::write(output, &archive)
        .map_err(|error| ArchiveError(format!("cannot write `{}`: {error}", output.display())))?;
    Ok(PackageArchive {
        path: output.to_owned(),
        checksum: fnv1a_checksum(&archive),
        files: files.into_iter().map(|(relative, _)| relative).collect(),
    })
}

pub fn verify_archive_checksum(path: &Path, expected: &str) -> Result<(), ArchiveError> {
    let bytes = fs::read(path)
        .map_err(|error| ArchiveError(format!("cannot read `{}`: {error}", path.display())))?;
    let actual = fnv1a_checksum(&bytes);
    if actual != expected {
        return Err(ArchiveError(format!(
            "ArchiveChecksumMismatch: expected `{expected}`, found `{actual}`"
        )));
    }
    Ok(())
}

pub fn fnv1a_checksum(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn collect_files(
    root: &Path,
    directory: &Path,
    output: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), ArchiveError> {
    let entries = fs::read_dir(directory).map_err(|error| {
        ArchiveError(format!("cannot inspect `{}`: {error}", directory.display()))
    })?;
    for entry in entries {
        let path = entry.map_err(|error| ArchiveError(error.to_string()))?.path();
        if should_ignore(&path, output) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, output, files)?;
        } else if path.is_file() {
            let relative = relative_path(root, &path)?;
            files.push((relative, path));
        }
    }
    Ok(())
}

fn should_ignore(path: &Path, output: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| matches!(name.to_str(), Some(".git" | "capsula" | "target")))
        || path == output
}

fn relative_path(root: &Path, path: &Path) -> Result<String, ArchiveError> {
    let relative = path.strip_prefix(root).map_err(|error| ArchiveError(error.to_string()))?;
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if normalized.len() > 100 {
        return Err(ArchiveError(format!(
            "archive path `{normalized}` exceeds the deterministic tar name limit"
        )));
    }
    Ok(normalized)
}

fn append_file(archive: &mut Vec<u8>, relative: &str, path: &Path) -> Result<(), ArchiveError> {
    let contents = fs::read(path)
        .map_err(|error| ArchiveError(format!("cannot read `{}`: {error}", path.display())))?;
    let mut header = [0_u8; BLOCK_SIZE];
    write_text(&mut header[0..100], relative);
    write_octal(&mut header[100..108], FILE_MODE.into());
    write_octal(&mut header[108..116], 0);
    write_octal(&mut header[116..124], 0);
    write_octal(&mut header[124..136], contents.len() as u64);
    write_octal(&mut header[136..148], 0);
    header[156] = b'0';
    write_text(&mut header[257..263], "ustar\0");
    write_text(&mut header[263..265], "00");
    write_checksum(&mut header);
    archive.extend_from_slice(&header);
    archive.extend_from_slice(&contents);
    let padding = (BLOCK_SIZE - contents.len() % BLOCK_SIZE) % BLOCK_SIZE;
    archive.resize(archive.len() + padding, 0);
    Ok(())
}

fn write_text(field: &mut [u8], text: &str) {
    let bytes = text.as_bytes();
    field[..bytes.len()].copy_from_slice(bytes);
}

fn write_octal(field: &mut [u8], value: u64) {
    let width = field.len() - 1;
    let text = format!("{value:0width$o}", width = width);
    field[..width].copy_from_slice(text.as_bytes());
    field[width] = b' ';
}

fn write_checksum(header: &mut [u8; BLOCK_SIZE]) {
    header[148..156].fill(b' ');
    let checksum = header.iter().map(|byte| u32::from(*byte)).sum::<u32>();
    let text = format!("{checksum:06o}\0 ");
    header[148..156].copy_from_slice(text.as_bytes());
}
