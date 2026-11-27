use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LONG: &str = "\"\"\"Close the owned Directory resource through an ins loan during best-effort early return, branch exit, and implicit scope cleanup.\"\"\"\nverb main() {}\n";
const WRAPPED: &str = "\"\"\"\nClose the owned Directory resource through an ins loan during best-effort early return, branch exit,\nand implicit scope cleanup.\n\"\"\"\nverb main() {}\n";

fn actus(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("run actus")
}

fn scratch(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("actus-fmt-docwrap-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("src")).expect("create scratch directory");
    fs::write(path.join("Actus.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n")
        .expect("write manifest");
    path
}

#[test]
fn check_reports_an_over_long_docstring_and_fmt_wraps_it() {
    let root = scratch("wrap");
    let file = root.join("src/main.act");
    fs::write(&file, LONG).expect("write source");

    let output = actus(&root, &["fmt", "--check", "src/main.act"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stderr), "src/main.act is not formatted\n");
    assert_eq!(fs::read_to_string(&file).expect("read"), LONG);

    let output = actus(&root, &["fmt", "src/main.act"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "formatted `src/main.act`\n");
    assert_eq!(fs::read_to_string(&file).expect("read"), WRAPPED);

    let output = actus(&root, &["fmt", "--check", "src/main.act"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "src/main.act is formatted\n");
    let _ = fs::remove_dir_all(root);
}
