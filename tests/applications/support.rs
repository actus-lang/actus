use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

pub(crate) fn copy_standard_library(root: &Path) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let files = [
        "lib.act",
        "io/io.act",
        "io/error.act",
        "io/reader.act",
        "io/writer.act",
        "io/stdout.act",
        "io/stderr.act",
        "io/stdin.act",
        "io/read.act",
        "io/write.act",
        "io/buffered.act",
        "io/cursor.act",
        "io/copy.act",
        "fs/fs.act",
        "fs/file.act",
        "fs/metadata.act",
        "fs/operations.act",
        "fs/options.act",
        "fs/seek.act",
        "path/path.act",
        "path/types.act",
        "path/storage.act",
        "path/error.act",
        "path/components.act",
        "path/posix.act",
        "path/windows.act",
        "path/predicates.act",
        "path/normalize.act",
        "path/builders.act",
    ];
    for relative in files {
        let destination = root.join("src").join(relative);
        fs::create_dir_all(destination.parent().expect("library parent"))
            .expect("library directory");
        fs::copy(source.join(relative), destination).expect("copy standard-library source");
    }
}

pub(crate) fn project(name: &str, source: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("actus-{name}-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create application project");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write application manifest");
    copy_standard_library(&root);
    let input = root.join("src/main.act");
    fs::write(&input, source).expect("write application source");
    (root.clone(), input, root.join("application"))
}

pub(crate) fn build(root: &Path, input: &Path, output: &Path) {
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(root)
        .output()
        .expect("check application");
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "exe", "-o"])
        .arg(output)
        .current_dir(root)
        .output()
        .expect("build application");
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
}

pub(crate) fn run(root: &Path, output: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(output)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run application");
    child
        .stdin
        .take()
        .expect("application stdin")
        .write_all(input)
        .expect("write application input");
    child.wait_with_output().expect("wait for application")
}
