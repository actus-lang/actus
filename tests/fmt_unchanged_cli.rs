use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

const FORMATTED: &str = "verb main() {\n    print(string: \"Hello, world!\");\n}\n";
const UNFORMATTED: &str = "verb   main()   {\nprint(string: \"Hello, world!\");\n}\n";

fn actus(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("run actus")
}

fn scratch(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("actus-fmt-unchanged-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("src")).expect("create scratch directory");
    fs::write(path.join("Actus.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n")
        .expect("write manifest");
    path
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn unchanged_file_is_not_rewritten_and_reports_already_formatted() {
    let root = scratch("unchanged");
    let file = root.join("src/main.act");
    fs::write(&file, FORMATTED).expect("write source");
    let before = fs::metadata(&file).expect("metadata").modified().expect("mtime");
    std::thread::sleep(Duration::from_millis(50));
    let output = actus(&root, &["fmt", "src/main.act"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "`src/main.act` is already formatted\n");
    assert_eq!(fs::read_to_string(&file).expect("read"), FORMATTED);
    assert_eq!(fs::metadata(&file).expect("metadata").modified().expect("mtime"), before);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn unformatted_file_is_rewritten_and_reports_formatted() {
    let root = scratch("changed");
    let file = root.join("src/main.act");
    fs::write(&file, UNFORMATTED).expect("write source");
    let output = actus(&root, &["fmt", "src/main.act"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "formatted `src/main.act`\n");
    assert_eq!(fs::read_to_string(&file).expect("read"), FORMATTED);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_wide_run_prints_a_summary_of_changed_and_unchanged_files() {
    let root = scratch("project");
    fs::write(root.join("src/main.act"), FORMATTED).expect("write main");
    fs::write(root.join("src/other.act"), UNFORMATTED).expect("write other");
    fs::write(root.join("src/third.act"), UNFORMATTED).expect("write third");
    let output = actus(&root, &["fmt"]);
    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    assert!(text.contains("is already formatted"), "{text}");
    let other = Path::new(".").join("src").join("other.act");
    assert!(text.contains(&format!("formatted `{}`", other.display())), "{text}");
    assert!(text.ends_with("formatted 2 files, 1 already formatted\n"), "{text}");
    fs::write(root.join("src/third.act"), UNFORMATTED).expect("rewrite third");
    let text = stdout(&actus(&root, &["fmt"]));
    assert!(text.ends_with("formatted 1 file, 2 already formatted\n"), "{text}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn check_keeps_its_messages_and_exit_codes_and_never_writes() {
    let root = scratch("check");
    let good = root.join("src/main.act");
    let bad = root.join("src/bad.act");
    fs::write(&good, FORMATTED).expect("write good");
    fs::write(&bad, UNFORMATTED).expect("write bad");
    let output = actus(&root, &["fmt", "--check", "src/main.act"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "src/main.act is formatted\n");
    let output = actus(&root, &["fmt", "--check", "src/bad.act"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stderr), "src/bad.act is not formatted\n");
    assert_eq!(fs::read_to_string(&bad).expect("read"), UNFORMATTED);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn suffixed_hex_literals_survive_formatting_unchanged() {
    let root = scratch("hex-suffix");
    let file = root.join("src/main.act");
    let source = "verb main() {\n    erg a: u8 = 0xFFu8;\n    erg b: i8 = 0x7fi8;\n    erg c: i40 = 0XaBi40;\n}\n";
    fs::write(&file, source).expect("write source");
    let output = actus(&root, &["fmt", "src/main.act"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&file).expect("read source"), source);
    assert_eq!(stdout(&output), "`src/main.act` is already formatted\n");
    let _ = fs::remove_dir_all(root);
}
