use super::support::{build, project, run};
use object::{Object, ObjectSymbol};
use std::fs;
use std::process::Command;

#[test]
fn module_wrapper_preserves_runtime_c_abi_across_object_boundaries() {
    let source = "import { open output }; verb main() -> Int { return emit(); }\n";
    let (root, input, output) = project("module-runtime-bridge", source);
    fs::create_dir_all(root.join("src/output")).expect("create output module directory");
    fs::write(
        root.join("src/output/output.act"),
        r#""""Re-export the output API."""
open api;
"#,
    )
    .expect("write output facade");
    fs::write(
        root.join("src/output/api.act"),
        r#""""The runtime bridge that prints an integer; the integer is passed by value."""
open unsafe extern "C" verb actus_print_int(erg value: Int) -> Int;

"""Print 79 through the runtime bridge and return 0."""
open verb emit() -> Int { erg value = 79; actus_print_int(value: value); return 0; }
"#,
    )
    .expect("write output implementation");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"79\n");
    assert_eq!(execution.stderr, b"");
    fs::remove_dir_all(root).expect("remove runtime bridge project");
}

#[cfg(unix)]
#[test]
fn unresolved_external_bridge_fails_during_multi_object_link() {
    let source = "import { open missing }; verb main() -> Int { return invoke(); }\n";
    let (root, input, output) = project("unresolved-bridge", source);
    fs::create_dir_all(root.join("src/missing")).expect("create missing module directory");
    fs::write(
        root.join("src/missing/missing.act"),
        r#""""Re-export the missing-bridge API."""
open api;
"#,
    )
    .expect("write missing facade");
    fs::write(
        root.join("src/missing/api.act"),
        r#""""A bridge to a native symbol that no object provides."""
open unsafe extern "C" verb absent_bridge() -> Int;

"""Call the absent bridge and return its result."""
open verb invoke() -> Int { return absent_bridge(); }
"#,
    )
    .expect("write unresolved bridge implementation");
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(&root)
        .output()
        .expect("check unresolved bridge application");
    assert!(check.status.success(), "check stderr: {}", String::from_utf8_lossy(&check.stderr));
    let build_result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "exe", "-o"])
        .arg(&output)
        .current_dir(&root)
        .output()
        .expect("build unresolved bridge application");
    assert!(!build_result.status.success());
    let diagnostics = String::from_utf8_lossy(&build_result.stderr);
    assert!(diagnostics.contains("absent_bridge"), "diagnostics: {diagnostics}");
    fs::remove_dir_all(root).expect("remove unresolved bridge project");
}

#[test]
fn freestanding_object_keeps_only_the_configured_entry_global() {
    let source = "import { open math }; verb boot() -> Int { return add(); }\n";
    let (root, input, output) = project("freestanding-objects", source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\nentry = \"boot\"\n\n[build]\nentry_contract = \"freestanding\"\n",
    )
    .expect("write freestanding manifest");
    fs::create_dir_all(root.join("src/math")).expect("create math module directory");
    fs::write(
        root.join("src/math/math.act"),
        r#""""Re-export the math API."""
open api;
"#,
    )
    .expect("write math facade");
    fs::write(
        root.join("src/math/api.act"),
        r#""""Return the constant 7."""
open verb add() -> Int { return 7; }
"#,
    )
    .expect("write math implementation");
    let object = output.with_extension("obj");
    let build_result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build freestanding object set");
    assert!(
        build_result.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&build_result.stderr)
    );
    assert!(!root.join("application.module.actus_mod_4_math.obj").exists());
    let bytes = fs::read(&object).expect("read program object");
    let file = object::File::parse(bytes.as_slice()).expect("parse program object");
    let global = file
        .symbols()
        .filter(|symbol| symbol.is_definition() && symbol.is_global())
        .filter_map(|symbol| symbol.name().ok())
        .collect::<Vec<_>>();
    assert!(
        global.len() == 1 && symbol_matches(global[0], "boot"),
        "the entry is the only global definition (ADR-0113): {global:?}"
    );
    assert!(file.symbols().any(|symbol| {
        symbol.is_definition()
            && !symbol.is_global()
            && symbol.name().is_ok_and(|name| symbol_matches(name, "actus_mod_4_math__verb_add"))
    }));
    fs::remove_dir_all(root).expect("remove freestanding object project");
}

#[test]
fn object_build_emits_one_object_for_the_program() {
    let source = "import { open math }; verb main() -> Int { return add(); }\n";
    let (root, input, output) = project("module-objects", source);
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write module facade");
    fs::write(
        root.join("src/math/api.act"),
        "open verb add() -> Int { return hidden(); } verb hidden() -> Int { return 42; }\n",
    )
    .expect("write module implementation");
    let object = output.with_extension("obj");
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build module objects");
    assert!(build.status.success(), "build stderr: {}", String::from_utf8_lossy(&build.stderr));
    assert!(object.exists());
    assert!(!root.join("application.module.actus_mod_4_math.obj").exists());
    let manifest = fs::read_to_string(object.with_extension("symbols")).expect("read manifest");
    assert_eq!(manifest.matches("[[objects]]").count(), 1, "manifest: {manifest}");
    fs::remove_dir_all(root).expect("remove module object project");
}

#[test]
fn module_application_rejects_private_bridge_before_codegen() {
    let source = "import { open runtime }; verb main() -> Int { return private_bridge(); }\n";
    let (root, input, _output) = project("private-bridge", source);
    fs::create_dir_all(root.join("src/runtime")).expect("create runtime module directory");
    fs::write(root.join("src/runtime/runtime.act"), "open api;\n").expect("write runtime facade");
    fs::write(
        root.join("src/runtime/api.act"),
        "open verb read() -> Int { return private_bridge(); } unsafe extern \"C\" verb private_bridge() -> Int;\n",
    )
    .expect("write runtime implementation");
    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", input.to_str().expect("source path"), "--strict"])
        .current_dir(&root)
        .output()
        .expect("check private bridge application");
    assert!(!check.status.success());
    let diagnostics = String::from_utf8_lossy(&check.stderr);
    assert!(diagnostics.contains("E1109"), "diagnostics: {diagnostics}");
    fs::remove_dir_all(root).expect("remove private bridge project");
}

fn symbol_matches(actual: &str, expected: &str) -> bool {
    actual == expected || actual.strip_prefix('_') == Some(expected)
}
