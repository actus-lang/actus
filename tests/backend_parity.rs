#![cfg(all(unix, feature = "cranelift", feature = "llvm"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use object::{Object, ObjectSymbol};

const BACKENDS: [&str; 2] = ["cranelift", "llvm"];
const SOURCE: &str = "import std::region; verb main() -> Int { erg values: Array[u8, 4] = Array[u8, 4](); values[0] = 41u8; return values[0] as Int + 1; } verb boot(erg backing: Buffer) -> Int { erg length: u64 = 1u64; erg start: u64 = 0u64; erg count: u64 = 1u64; erg opened = region_open[u8](backing: dat backing, logical_length: erg length, window_start: erg start, window_count: erg count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { drop(region); return 0; }, }; }\n";

fn fixture_root(name: &str, backend: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("actus-backend-parity-{name}-{backend}-{}", std::process::id()))
}

fn write_fixture(root: &Path, backend: &str, target: Option<&str>, source: &str) -> PathBuf {
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create parity fixture source directory");
    let target_line = target.map_or(String::new(), |value| format!("target = \"{value}\"\n"));
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"backend-parity\"\nversion = \"0.1.0\"\nsource_root = \"src\"\nentry = \"{}\"\n\n[build]\nruntime = \"std\"\nbackend = \"{backend}\"\n{target_line}entry_contract = \"{}\"\nverify_no_float_ir = true\n",
            if target.is_some() { "boot" } else { "main" },
            if target.is_some() { "freestanding" } else { "hosted" }
        ),
    )
    .expect("write parity fixture manifest");
    let input = source_root.join("main.act");
    fs::write(&input, source).expect("write parity fixture source");
    input
}

fn build(root: &Path, input: &Path, output: &Path, emit: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            input.to_str().expect("fixture path should be UTF-8"),
            "--strict",
            "--emit",
            emit,
            "-o",
            output.to_str().expect("output path should be UTF-8"),
        ])
        .current_dir(root)
        .output()
        .expect("Actus parity build should start")
}

fn symbols(bytes: &[u8], defined: bool) -> BTreeSet<String> {
    let file = object::File::parse(bytes).expect("parity object should parse");
    file.symbols()
        .filter(|symbol| if defined { symbol.is_definition() } else { symbol.is_undefined() })
        .filter_map(|symbol| symbol.name().ok().map(str::to_owned))
        .filter(|name| name == "boot" || name.starts_with("actus_"))
        .collect()
}

#[test]
fn hosted_cli_backends_preserve_exit_status_and_buffer_ownership() {
    let mut statuses = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("hosted", backend);
        let input = write_fixture(&root, backend, None, SOURCE);
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} hosted build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run hosted parity fixture");
        statuses.push(status.code());
        assert_eq!(status.code(), Some(42), "{backend} hosted runtime result");
        fs::remove_dir_all(root).expect("remove hosted parity fixture");
    }
    assert_eq!(statuses, vec![Some(42), Some(42)]);
}

#[test]
fn freestanding_cli_backends_preserve_symbols_and_zero_float_evidence() {
    let mut defined = Vec::new();
    let mut undefined = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("freestanding", backend);
        let input = write_fixture(&root, backend, Some("x86_64-unknown-uefi"), SOURCE);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(
            build.status.success(),
            "{backend} freestanding build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let stdout = String::from_utf8_lossy(&build.stdout);
        let stderr = String::from_utf8_lossy(&build.stderr);
        assert!(
            stdout.contains("no floating-point instructions")
                || stderr.contains("no floating-point instructions"),
            "{backend} did not report zero-float verification; stdout={stdout} stderr={stderr}"
        );
        let bytes = fs::read(&output).expect("read freestanding parity object");
        defined.push(symbols(&bytes, true));
        undefined.push(symbols(&bytes, false));
        fs::remove_dir_all(root).expect("remove freestanding parity fixture");
    }
    assert!(defined[0].contains("boot"), "freestanding entry symbol missing: {:?}", defined[0]);
    assert_eq!(defined[0], defined[1], "freestanding exported symbols diverged");
    assert_eq!(undefined[0], undefined[1], "runtime bridge symbols diverged");
}

#[test]
fn freestanding_cli_backends_share_large_aggregate_failure_class() {
    let source = "struct Cell { erg value: u8, } verb boot() -> Int { erg cells: Array[Cell, 131072] = Array[Cell, 131072](); return cells[0].value as Int; }\n";
    for backend in BACKENDS {
        let root = fixture_root("failure", backend);
        let input = write_fixture(&root, backend, Some("thumbv7em-none-eabihf"), source);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(!build.status.success(), "{backend} must reject implicit allocation");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        assert!(
            diagnostics.contains("implicit dynamic allocation")
                || diagnostics.contains("implicit heap allocation"),
            "{backend} failure class changed: {diagnostics}"
        );
        fs::remove_dir_all(root).expect("remove failure parity fixture");
    }
}
