use super::support::{build, project, run};
use object::{Object, ObjectSymbol};
use std::fs;
use std::process::Command;
#[test]
fn package_configuration_module_emits_without_an_anchor_verb() {
    let source = "import config; verb main() -> Int { return LIMIT as Int; }\n";
    let (root, input, output) = project("const-only-config", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const LIMIT: u8 = 30u8;\n")
        .expect("write configuration values");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(30));
    let object = output.with_extension("obj");
    let object_build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build const-only configuration objects");
    assert!(
        object_build.status.success(),
        "object build stderr: {}",
        String::from_utf8_lossy(&object_build.stderr)
    );
    let module_bytes = fs::read(root.join("application.module.actus_mod_6_config.obj"))
        .expect("read const-only configuration object");
    let module_file = object::File::parse(module_bytes.as_slice())
        .expect("parse const-only configuration object");
    assert!(
        !module_file
            .symbols()
            .filter_map(|symbol| symbol.name().ok())
            .any(|symbol| symbol.contains("LIMIT"))
    );
    fs::remove_dir_all(root).expect("remove const-only configuration project");
}

#[test]
fn package_configuration_supports_all_compile_time_consumers() {
    let source = "import config; struct Settings { erg threshold: u8, } struct Storage[N: Usize] { erg values: Array[u8, N], } verb main() -> Int { erg values: Array[u8, 2] = Array[u8, 2](); values[0] = LIMIT; erg settings = Settings { threshold: values[0], }; erg storage: Storage[2] = Storage[2] { values: Array[u8, 2](), }; storage.values[0] = settings.threshold; erg result = storage.values[0] as Int; if ENABLED == true { return result; } return 2; }\n";
    let (root, input, output) = project("config-acceptance", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(
        root.join("src/config/values.act"),
        "open const LIMIT: u8 = 30u8; open const ENABLED: Bool = true;\n",
    )
    .expect("write configuration values");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(30));
    fs::remove_dir_all(root).expect("remove configuration acceptance project");
}

#[test]
fn imported_module_can_lower_a_configuration_constant() {
    let source = "import worker; verb main() -> Int { return read_magic(); }\n";
    let (root, input, output) = project("config-imported-module", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BRAIN_MAGIC: u32 = 42u32;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/worker")).expect("create worker module");
    fs::write(root.join("src/worker/worker.act"), "open api;\n").expect("write worker facade");
    fs::write(
        root.join("src/worker/api.act"),
        "import config; open verb read_magic() -> Int { return BRAIN_MAGIC as Int; }\n",
    )
    .expect("write worker implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    fs::remove_dir_all(root).expect("remove imported configuration project");
}

#[test]
fn nested_imported_module_can_lower_a_configuration_constant() {
    let source = "import aie; verb main() -> Int { return read_magic(); }\n";
    let (root, input, output) = project("config-nested-imported-module", source);
    fs::create_dir_all(root.join("src/config")).expect("create configuration directory");
    fs::write(root.join("src/config/config.act"), "open values;\n")
        .expect("write configuration facade");
    fs::write(root.join("src/config/values.act"), "open const BRAIN_MAGIC: u32 = 42u32;\n")
        .expect("write configuration values");
    fs::create_dir_all(root.join("src/aie/persistence")).expect("create nested module");
    fs::write(root.join("src/aie/aie.act"), "open persistence;\n").expect("write aie facade");
    fs::write(root.join("src/aie/persistence/persistence.act"), "open serialization;\n")
        .expect("write persistence facade");
    fs::write(
        root.join("src/aie/persistence/serialization.act"),
        "import config; open verb read_magic() -> Int { return BRAIN_MAGIC as Int; }\n",
    )
    .expect("write serialization implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(42));
    fs::remove_dir_all(root).expect("remove nested configuration project");
}

#[test]
fn canonical_parent_facade_exposes_nested_configuration_constant() {
    let source = "import feature; verb main() -> Int { return stride() as Int; }\n";
    let (root, input, output) = project("canonical-constant-facade", source);
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

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(24));
    fs::remove_dir_all(root).expect("remove canonical constant project");
}

#[test]
fn nested_facade_constant_has_object_and_executable_parity() {
    let source = "import feature; verb main() -> Int { return inspect() as Int; }\n";
    let (root, input, output) = project("constant-native-parity", source);
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

open verb inspect() -> u16 {
    erg settings = Settings { stride: BUFFER_STRIDE, };
    if BUFFER_STRIDE == 24u16 {
        return settings.stride;
    }
    return 0u16;
}
"#,
    )
    .expect("write feature implementation");

    build(&root, &input, &output);
    let execution = run(&root, &output, b"");
    assert_eq!(execution.status.code(), Some(24));

    let object = output.with_extension("obj");
    let object_build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().expect("source path"), "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build native objects");
    assert!(
        object_build.status.success(),
        "object build stderr: {}",
        String::from_utf8_lossy(&object_build.stderr)
    );
    let feature_object = fs::read_dir(&root)
        .expect("read project outputs")
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_name().to_string_lossy().contains("feature")
                && entry.path().extension().is_some_and(|extension| extension == "obj")
        })
        .expect("feature object");
    let bytes = fs::read(feature_object.path()).expect("read feature object");
    let object_file = object::File::parse(bytes.as_slice()).expect("parse feature object");
    assert!(
        !object_file
            .symbols()
            .filter_map(|symbol| symbol.name().ok())
            .any(|symbol| symbol.contains("BUFFER_STRIDE"))
    );
    fs::remove_dir_all(root).expect("remove native parity project");
}
