#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
#[test]
fn project_workflow_runs_console_application_through_std_io() {
    let root = std::env::temp_dir().join(format!("actus-console-project-{}", std::process::id()));
    let output = root.join("console");
    let standard_library = Path::new(env!("CARGO_MANIFEST_DIR")).join("library/std");
    fs::create_dir_all(root.join("src")).expect("create project source directory");
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"console\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[dependencies]\nio = {{ path = \"{}\" }}\n",
            standard_library.display()
        ),
    )
    .expect("write project manifest");
    fs::write(
        root.join("src/main.act"),
        "import io; verb main() -> Int { erg text = Buffer[0]; append(text, 79); append(text, 75); println(text: abs text); eprintln(text: abs text); return 9; }\n",
    )
    .expect("write console application");

    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("check")
        .current_dir(&root)
        .output()
        .expect("run project check");
    assert!(check.status.success(), "stderr: {}", String::from_utf8_lossy(&check.stderr));

    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&root)
        .output()
        .expect("build console application");
    assert!(build.status.success(), "stderr: {}", String::from_utf8_lossy(&build.stderr));
    let built = Command::new(&output).output().expect("run built console application");
    assert_eq!(built.status.code(), Some(9));
    assert_eq!(built.stdout, b"OK\n");
    assert_eq!(built.stderr, b"OK\n");

    let run = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("run")
        .current_dir(&root)
        .output()
        .expect("run project console application");
    assert_eq!(run.status.code(), Some(9));
    assert_eq!(run.stdout, b"OK\n");
    assert!(String::from_utf8_lossy(&run.stderr).contains("process exited with status 9"));
    let _ = fs::remove_dir_all(root);
}
