use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn actus(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_actus")).args(arguments).output().expect("run actus")
}

fn scratch(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("actus-unknown-option-{name}-{}", std::process::id()));
    fs::create_dir_all(&path).expect("create scratch directory");
    path
}

const COMMANDS: [&str; 14] = [
    "check",
    "build",
    "run",
    "fmt",
    "watch",
    "test",
    "lock",
    "new",
    "init",
    "publish",
    "lsp",
    "version",
    "conformance",
    "parse",
];

#[test]
fn every_subcommand_rejects_an_unknown_option_naming_option_and_command() {
    let directory = scratch("reject");
    for command in COMMANDS {
        for option in ["--bogus", "-z"] {
            let output = Command::new(env!("CARGO_BIN_EXE_actus"))
                .args([command, option])
                .current_dir(&directory)
                .output()
                .expect("run actus");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(2), "`{command} {option}` stderr: {stderr}");
            let expected = format!("error: unknown option `{option}` for `actus {command}`");
            assert!(stderr.contains(&expected), "`{command} {option}` stderr: {stderr}");
        }
    }
    assert_eq!(fs::read_dir(&directory).expect("list scratch").count(), 0, "no side effects");
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn unknown_option_is_rejected_after_a_valid_input_file() {
    let directory = scratch("after-input");
    let source = directory.join("main.act");
    fs::write(&source, "verb main() -> Int { return 42; }\n").expect("write source");
    let source = source.to_str().expect("utf-8 path");
    for command in ["check", "build", "run", "fmt", "watch", "parse", "publish"] {
        let output = actus(&[command, source, "--bogus"]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "`{command}` stderr: {stderr}");
        assert!(stderr.contains(&format!("unknown option `--bogus` for `actus {command}`")));
    }
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn strict_is_rejected_by_commands_without_it_and_hints_at_supporting_commands() {
    let directory = scratch("strict");
    for command in ["run", "fmt", "watch", "lock", "init", "publish", "lsp", "version", "parse"] {
        let output = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args([command, "--strict"])
            .current_dir(&directory)
            .output()
            .expect("run actus");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "`{command}` stderr: {stderr}");
        assert!(
            stderr.contains(&format!(
                "error: unknown option `--strict` for `actus {command}`; `--strict` is an option of `actus check`, `actus build`, `actus test` and `actus conformance`"
            )),
            "`{command}` stderr: {stderr}"
        );
    }
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn supported_options_still_work() {
    let directory = scratch("valid");
    let source = directory.join("main.act");
    fs::write(&source, "verb main() -> Int {\n    return 42;\n}\n").expect("write source");
    let source = source.to_str().expect("utf-8 path");

    assert!(actus(&["check", source, "--strict"]).status.success());
    assert!(actus(&["fmt", source, "--check"]).status.success());
    assert!(actus(&["parse", source]).status.success());
    assert!(actus(&["version", "--json"]).status.success());
    assert!(actus(&["--version", "--json"]).status.success());
    let watch = actus(&["watch", source, "--once", "--interval", "50"]);
    assert!(watch.status.success(), "stderr: {}", String::from_utf8_lossy(&watch.stderr));
    let listed = actus(&["test", "--list", "--strict"]);
    assert_ne!(
        listed.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&listed.stderr)
    );

    let created = directory.join("fresh");
    let new = actus(&["new", created.to_str().expect("utf-8 path"), "--no-git"]);
    assert!(new.status.success(), "stderr: {}", String::from_utf8_lossy(&new.stderr));
    let _ = fs::remove_dir_all(directory);
}

#[cfg(unix)]
#[test]
fn run_forwards_option_like_arguments_after_the_separator() {
    let directory = scratch("run-separator");
    let source = directory.join("main.act");
    fs::write(&source, "verb main() -> Int { return 42; }\n").expect("write source");
    let source = source.to_str().expect("utf-8 path");

    let output = actus(&["run", source, "--release", "--", "--strict", "-z"]);
    assert_eq!(
        output.status.code(),
        Some(42),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(directory);
}
