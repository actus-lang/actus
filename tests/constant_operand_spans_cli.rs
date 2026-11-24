//! A diagnostic on a constant operand points at the use, not at the
//! constant's declaration, in the file that contains the use. The driver
//! checks the program after inlining constants, so the inlined value has to
//! keep the span of the name it replaces.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn project(name: &str, sources: &[(&str, &str)]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("actus-constant-spans-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"spans-{name}\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n"
        ),
    )
    .expect("write manifest");
    fs::write(root.join("Actus.lock"), "lockfile_version = 3\nruntime = \"core\"\npackage = []\n")
        .expect("write lockfile");
    for (path, source) in sources {
        let path = root.join("src").join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
        fs::write(path, source).expect("write source");
    }
    root
}

fn check_stderr(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", "src/main.act"])
        .current_dir(root)
        .output()
        .expect("run actus check");
    assert!(!output.status.success(), "the fixture must be rejected");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_local_constant_operand_is_reported_at_the_use() {
    let main = "const LIMIT: u16 = 48u16;\n\nverb main() {\n    erg a: u32 = 5u32;\n    if a == LIMIT {\n        print(int: 1);\n    }\n}\n";
    let root = project("local-eq", &[("main.act", main)]);
    let stderr = check_stderr(&root);
    assert!(stderr.contains("src/main.act: error[E1140] at 5:13:"), "{stderr}");

    let main = "const LIMIT: u16 = 48u16;\n\nverb main() {\n    erg a: u32 = 5u32;\n    erg b = a + LIMIT;\n}\n";
    let root = project("local-add", &[("main.act", main)]);
    let stderr = check_stderr(&root);
    assert!(stderr.contains("src/main.act: error[E1140] at 5:17:"), "{stderr}");
}

#[test]
fn a_signedness_mismatch_on_a_constant_is_reported_at_the_use() {
    let main = "const NEGATIVE: i8 = -1i8;\n\nverb main() {\n    erg a: u8 = 5u8;\n    erg b = a + NEGATIVE;\n}\n";
    let root = project("local-range", &[("main.act", main)]);
    let stderr = check_stderr(&root);
    assert!(stderr.contains("src/main.act: error[E1140] at 5:17:"), "{stderr}");
}

#[test]
fn a_constant_from_another_module_is_reported_in_the_using_file() {
    let main = "import config;\n\nverb main() {\n    erg a: u32 = 5u32;\n    if a == config::X {\n        print(int: 1);\n    }\n    erg b = a + config::X;\n}\n";
    let root = project(
        "module",
        &[
            ("main.act", main),
            ("config/config.act", "open values;\n"),
            ("config/values.act", "open const X: u16 = 48u16;\n"),
        ],
    );
    let stderr = check_stderr(&root);
    assert!(stderr.contains("src/main.act: error[E1140] at 5:13:"), "{stderr}");
    assert!(!stderr.contains("values.act"), "{stderr}");
}
