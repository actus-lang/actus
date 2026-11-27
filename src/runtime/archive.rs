use std::path::PathBuf;

/// Finds the runtime archive. The error text carries no diagnostic code: the
/// runtime crate is also built on its own, so the compiler adds the code.
pub fn runtime_archive_path() -> Result<Option<PathBuf>, String> {
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        let install_root = parent.join("..").canonicalize().ok();
        if let Some(install_root) = install_root {
            let marker = install_root.join("share/actus/toolchain.json");
            if marker.is_file() {
                let installed = install_root.join("lib/actus/runtime").join(runtime_host_triple());
                let archive = std::fs::read_dir(&installed)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .find(|path| {
                        path.is_file()
                            && path.file_stem().is_some_and(|name| name == "libactus_runtime")
                    });
                return archive.map(Some).ok_or_else(|| {
                    format!("installed runtime archive is missing under `{}`", installed.display())
                });
            }
        }
    }
    Ok(option_env!("ACTUS_RUNTIME_ARCHIVE").map(PathBuf::from))
}

fn runtime_host_triple() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => "unknown",
    }
}
