#![cfg(all(unix, feature = "cranelift", feature = "llvm"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use object::{BinaryFormat, Object, ObjectSection, ObjectSymbol, RelocationTarget};

const BACKENDS: [&str; 2] = ["cranelift", "llvm"];
const SOURCE: &str = "import std::region; verb main() -> Int { erg values: Array[u8, 4] = Array[u8, 4](); values[0] = 41u8; return values[0] as Int + 1; } verb boot(erg backing: Buffer) -> Int { erg length: u64 = 1u64; erg start: u64 = 0u64; erg count: u64 = 1u64; erg opened = region_open[u8](backing: dat backing, logical_length: erg length, window_start: erg start, window_count: erg count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { drop(region); return 0; }, }; }\n";

fn fixture_root(name: &str, backend: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("actus-backend-parity-{name}-{backend}-{}", std::process::id()))
}

fn write_fixture(
    root: &Path,
    backend: &str,
    target: Option<&str>,
    entry_contract: Option<&str>,
    source: &str,
) -> PathBuf {
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create parity fixture source directory");
    let target_line = target.map_or(String::new(), |value| format!("target = \"{value}\"\n"));
    let entry_line =
        entry_contract.map_or(String::new(), |value| format!("entry_contract = \"{value}\"\n"));
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"backend-parity\"\nversion = \"0.1.0\"\nsource_root = \"src\"\nentry = \"{}\"\n\n[build]\nruntime = \"std\"\nbackend = \"{backend}\"\n{target_line}{entry_line}verify_no_float_ir = true\n",
            if target.is_some() || entry_contract.is_some() { "boot" } else { "main" },
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

fn canonical_symbol_name(format: BinaryFormat, name: &str) -> String {
    if format == BinaryFormat::MachO {
        name.strip_prefix('_').unwrap_or(name).to_owned()
    } else {
        name.to_owned()
    }
}

fn symbols(bytes: &[u8], defined: bool) -> BTreeSet<String> {
    let file = object::File::parse(bytes).expect("parity object should parse");
    let format = file.format();
    file.symbols()
        .filter(|symbol| if defined { symbol.is_definition() } else { symbol.is_undefined() })
        .filter_map(|symbol| symbol.name().ok().map(|name| canonical_symbol_name(format, name)))
        .filter(|name| name == "boot" || name.starts_with("actus_"))
        .collect()
}

fn referenced_undefined_symbols(bytes: &[u8]) -> BTreeSet<String> {
    let file = object::File::parse(bytes).expect("parity object should parse");
    let format = file.format();
    let mut symbols = BTreeSet::new();
    for section in file.sections() {
        for (_, relocation) in section.relocations() {
            let RelocationTarget::Symbol(index) = relocation.target() else { continue };
            let symbol = file.symbol_by_index(index).expect("relocation symbol should exist");
            if symbol.is_undefined()
                && let Ok(name) = symbol.name()
                && let name = canonical_symbol_name(format, name)
                && name.starts_with("actus_")
            {
                symbols.insert(name);
            }
        }
    }
    symbols
}

#[test]
fn hosted_cli_backends_preserve_exit_status_and_buffer_ownership() {
    let mut statuses = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("hosted", backend);
        let input = write_fixture(&root, backend, None, None, SOURCE);
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
fn hosted_cli_backends_accept_named_array_capacity_constants() {
    let source = "const CAPACITY: Usize = 8u64 as Usize; verb main() -> Int { erg values: Array[u8, CAPACITY] = Array[u8, CAPACITY](); values[0] = 41u8; return values[0] as Int + 1; }\n";
    for backend in BACKENDS {
        let root = fixture_root("named-array-capacity", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} named array capacity build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status =
            Command::new(&output).status().expect("run named array capacity parity fixture");
        assert_eq!(status.code(), Some(42), "{backend} named array capacity result");
        fs::remove_dir_all(root).expect("remove named array capacity fixture");
    }
}

#[test]
fn hosted_cli_backends_share_float_constants_and_comparisons() {
    let source = "verb main() -> Int { erg temperature: f32 = 21.5; erg pressure: f64 = 1013.25; return case temperature < 22.0f32 && pressure > 1000.0 { true => 42, _ => 1, }; }\n";
    for backend in BACKENDS {
        let root = fixture_root("float-hosted", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let manifest = root.join("Actus.toml");
        let contents = fs::read_to_string(&manifest).expect("read float parity manifest");
        fs::write(
            &manifest,
            contents.replace("verify_no_float_ir = true", "verify_no_float_ir = false"),
        )
        .expect("disable zero-float audit for hosted float parity");
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} hosted float build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run hosted float parity fixture");
        assert_eq!(status.code(), Some(42), "{backend} hosted float runtime result");
        fs::remove_dir_all(root).expect("remove hosted float fixture");
    }
}

#[test]
fn freestanding_cli_backends_preserve_symbols_and_zero_float_evidence() {
    let mut defined = Vec::new();
    let mut undefined = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("freestanding", backend);
        let input = write_fixture(&root, backend, None, Some("freestanding"), SOURCE);
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
        undefined.push(referenced_undefined_symbols(&bytes));
        fs::remove_dir_all(root).expect("remove freestanding parity fixture");
    }
    assert!(defined[0].contains("boot"), "freestanding entry symbol missing: {:?}", defined[0]);
    assert_eq!(defined[0], defined[1], "freestanding exported symbols diverged");
    assert!(
        undefined[0].contains("actus_region_drop")
            && undefined[0].iter().any(|symbol| symbol.contains("__verb_region_5fopen_5f_5fu8")),
        "required Region runtime bridges are missing: {:?}",
        undefined[0]
    );
    assert_eq!(undefined[0], undefined[1], "runtime bridge relocations diverged");
}

#[test]
fn freestanding_cli_backends_share_large_aggregate_failure_class() {
    let source = "struct Cell { erg value: u8, } verb boot() -> Int { erg cells: Array[Cell, 131072] = Array[Cell, 131072](); return cells[0].value as Int; }\n";
    for backend in BACKENDS {
        let root = fixture_root("failure", backend);
        let input = write_fixture(&root, backend, Some("thumbv7em-none-eabihf"), None, source);
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

#[test]
fn freestanding_cli_backends_reject_float_at_shared_target_boundary() {
    let source = "verb boot() -> Int { erg sample: f32 = 1.5f32; return case sample > 1.0f32 { true => 0, _ => 1, }; }\n";
    for backend in BACKENDS {
        let root = fixture_root("float-freestanding", backend);
        let input = write_fixture(&root, backend, None, Some("freestanding"), source);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(!build.status.success(), "{backend} must reject freestanding float");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        assert!(
            diagnostics
                .contains("floating-point values are forbidden by the freestanding target profile"),
            "{backend} float policy diagnostic changed: {diagnostics}"
        );
        fs::remove_dir_all(root).expect("remove freestanding float fixture");
    }
}
