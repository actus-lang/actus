use super::support::{build, project, run};
use object::{Object, ObjectSymbol};
use std::fs;
use std::process::Command;

#[test]
fn nested_runtime_facade_qualifies_sibling_imports() {
    let source = "import feature; verb main() -> Int { return runtime_probe(); }\n";
    let (root, input, output) = project("nested-runtime-sibling-import", source);
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"application\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("enable standard runtime");
    fs::create_dir_all(root.join("src/feature")).expect("create feature module");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "import std::fs; open verb runtime_probe() -> Int { return 0; }\n",
    )
    .expect("write runtime import fixture");
    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(0));
    fs::remove_dir_all(root).expect("remove nested runtime import project");
}

#[test]
fn generic_nested_module_lowers_facade_constant_without_native_binding() {
    let source = "import feature; verb main() -> Int { return inspect[2]() as Int; }\n";
    let (root, input, output) = project("generic-constant-facade", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BUFFER_STRIDE: u16 = 24u16;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "import config; open verb inspect[N: Usize]() -> u16 { return BUFFER_STRIDE + (N as u16); }\n",
    )
    .expect("write generic feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(26));
    fs::remove_dir_all(root).expect("remove generic constant project");
}

#[test]
fn generic_facade_constant_survives_scalar_predicate_and_aggregate_specialization() {
    let source = "import feature; verb main() -> Int { return inspect[2]() as Int; }\n";
    let (root, input, output) = project("generic-constant-uses", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BUFFER_STRIDE: u16 = 24u16;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        r#"import config;

open struct Settings {
    erg stride: u16,
}

open verb inspect[N: Usize]() -> u16 {
    erg scalar: u16 = BUFFER_STRIDE;
    erg settings = Settings { stride: BUFFER_STRIDE, };
    if scalar == BUFFER_STRIDE && (N as u16) == 2u16 {
        return settings.stride + (N as u16);
    }
    return 0u16;
}
"#,
    )
    .expect("write generic feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(26));
    fs::remove_dir_all(root).expect("remove generic constant uses project");
}

#[test]
fn facade_constant_objects_are_deterministic_and_symbol_free() {
    let source = "import feature; verb main() -> Int { return stride() as Int; }\n";
    let (root, input, output) = project("constant-symbol-integrity", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BUFFER_STRIDE: u16 = 24u16;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/feature")).expect("create feature directory");
    fs::write(root.join("src/feature/feature.act"), "open implementation;\n")
        .expect("write feature facade");
    fs::write(
        root.join("src/feature/implementation.act"),
        "import config; open verb stride() -> u16 { return BUFFER_STRIDE; }\n",
    )
    .expect("write feature implementation");

    let first = output.with_extension("first.obj");
    let second = output.with_extension("second.obj");
    for object in [&first, &second] {
        let result = Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
            .arg(object)
            .current_dir(&root)
            .output()
            .expect("build deterministic object");
        assert!(
            result.status.success(),
            "object build stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(
        fs::read(&first).expect("read first object"),
        fs::read(&second).expect("read second object")
    );

    for entry in fs::read_dir(&root).expect("read object outputs").flatten() {
        if entry.path().extension().is_none_or(|extension| extension != "obj") {
            continue;
        }
        let bytes = fs::read(entry.path()).expect("read emitted object");
        let object = object::File::parse(bytes.as_slice()).expect("parse emitted object");
        let symbols = object.symbols().filter_map(|symbol| symbol.name().ok()).collect::<Vec<_>>();
        let unique = symbols.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(symbols.len(), unique.len(), "duplicate symbols in {:?}", entry.path());
        assert!(!symbols.iter().any(|symbol| symbol.contains("BUFFER_STRIDE")));
    }
    fs::remove_dir_all(root).expect("remove symbol integrity project");
}
