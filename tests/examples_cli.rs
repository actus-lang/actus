#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;
#[cfg(unix)]
use object::{Object, ObjectSymbol};

#[cfg(unix)]
fn has_main_symbol(object_file: &object::File<'_>) -> bool {
    object_file
        .symbols()
        .any(|symbol| symbol.name().is_ok_and(|name| matches!(name, "main" | "_main")))
}

#[cfg(unix)]
#[test]
fn sensor_telemetry_example_builds_and_executes() {
    let root = std::env::temp_dir().join(format!("actus-sensor-example-{}", std::process::id()));
    let output = root.with_extension("bin");
    let source = format!("{}/examples/sensor_telemetry.act", env!("CARGO_MANIFEST_DIR"));
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source,
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run sensor telemetry example");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn hardware_register_example_builds_and_executes() {
    let root = std::env::temp_dir().join(format!("actus-register-example-{}", std::process::id()));
    let output = root.with_extension("bin");
    let source = format!("{}/examples/hardware_register.act", env!("CARGO_MANIFEST_DIR"));
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source,
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run hardware register example");
    assert_eq!(status.code(), Some(8));
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn arena_tree_example_builds_and_executes() {
    let root =
        std::env::temp_dir().join(format!("actus-arena-tree-example-{}", std::process::id()));
    let output = root.with_extension("bin");
    let source = format!("{}/examples/arena_tree.act", env!("CARGO_MANIFEST_DIR"));
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source,
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run arena tree example");
    assert_eq!(status.code(), Some(60));
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn phase18_arrays_and_packs_example_builds_and_executes() {
    let root = std::env::temp_dir().join(format!("actus-phase18-example-{}", std::process::id()));
    let output = root.with_extension("bin");
    let source = format!("{}/examples/phase18_arrays_and_packs.act", env!("CARGO_MANIFEST_DIR"));
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source,
            "--strict".to_owned(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run phase 18 systems example");
    assert_eq!(status.code(), Some(92));
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn runtime_profile_example_builds_and_executes_with_builtin_std() {
    let root = std::env::temp_dir().join(format!("actus-runtime-profile-{}", std::process::id()));
    let output = root.with_extension("bin");
    let source = format!("{}/examples/runtime_profiles/src/main.act", env!("CARGO_MANIFEST_DIR"));
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source,
            "--strict".to_owned(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let execution =
        std::process::Command::new(&output).output().expect("run runtime profile example");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(execution.stdout, b"text outputtext output\nbytesbytes\n");
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn phase23_capability_package_passes_strict_test_native_and_object_acceptance() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/phase23_capability");
    let binary = std::env::temp_dir().join(format!("actus-phase23-{}.bin", std::process::id()));
    let object = std::env::temp_dir().join(format!("actus-phase23-{}.o", std::process::id()));
    let rejected = root.join("tests/fixtures/rejected_runtime_const.act");

    for arguments in [vec!["check", "--strict"], vec!["test", "--strict"]] {
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(arguments)
            .current_dir(&root)
            .output()
            .expect("run strict Phase 23 package command");
        assert!(result.status.success(), "strict command failed: {:?}", result);
    }

    let formatting = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["fmt", "--check"])
        .current_dir(&root)
        .output()
        .expect("check Phase 23 package formatting");
    assert!(formatting.status.success(), "format check failed: {:?}", formatting);

    let build = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&binary)
        .current_dir(&root)
        .output()
        .expect("build Phase 23 capability executable");
    assert!(build.status.success(), "native build failed: {:?}", build);
    let execution =
        std::process::Command::new(&binary).output().expect("run Phase 23 capability executable");
    assert_eq!(execution.status.code(), Some(42));

    let object_build = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build Phase 23 capability object");
    assert!(object_build.status.success(), "object build failed: {:?}", object_build);
    let object_bytes = std::fs::read(&object).expect("read Phase 23 object");
    let object_file = object::File::parse(object_bytes.as_slice()).expect("parse Phase 23 object");
    assert!(object_file.section_by_name(".text").is_some());
    assert!(has_main_symbol(&object_file));

    let rejected_check = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["check", rejected.to_str().expect("rejected fixture path"), "--strict"])
        .current_dir(&root)
        .output()
        .expect("check rejected Phase 23 fixture");
    assert!(!rejected_check.status.success());

    let _ = std::fs::remove_file(binary);
    let _ = std::fs::remove_file(object);
}

#[cfg(unix)]
#[test]
fn phase23_readiness_package_passes_strict_and_native_acceptance() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/phase23_readiness");
    let binary =
        std::env::temp_dir().join(format!("actus-phase23-readiness-{}.bin", std::process::id()));
    let object =
        std::env::temp_dir().join(format!("actus-phase23-readiness-{}.o", std::process::id()));
    let rejected = [
        "rejected_buffer_owner.act",
        "rejected_pack_overlap.act",
        "rejected_pack_bounds.act",
        "rejected_pack_element.act",
        "rejected_pack_runtime_capacity.act",
        "rejected_pack_index_bounds.act",
        "rejected_pack_ownership.act",
    ]
    .map(|name| root.join("tests/fixtures").join(name));

    for arguments in [vec!["check", "--strict"], vec!["test", "--strict"], vec!["fmt", "--check"]] {
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(arguments)
            .current_dir(&root)
            .output()
            .expect("run strict Phase 23.14 package command");
        assert!(result.status.success(), "readiness command failed: {:?}", result);
    }

    let build = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--strict", "--emit", "exe", "-o"])
        .arg(&binary)
        .current_dir(&root)
        .output()
        .expect("build Phase 23.14 readiness executable");
    assert!(build.status.success(), "readiness build failed: {:?}", build);
    let execution =
        std::process::Command::new(&binary).output().expect("run Phase 23.14 readiness executable");
    assert_eq!(execution.status.code(), Some(126));

    let object_build = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&object)
        .current_dir(&root)
        .output()
        .expect("build Phase 23.14 readiness object");
    assert!(object_build.status.success(), "readiness object failed: {:?}", object_build);
    let object_bytes = std::fs::read(&object).expect("read Phase 23.14 object");
    let object_file =
        object::File::parse(object_bytes.as_slice()).expect("parse Phase 23.14 object");
    assert!(object_file.section_by_name(".text").is_some());
    assert!(has_main_symbol(&object_file));

    for fixture in rejected {
        let rejected_check = std::process::Command::new(env!("CARGO_BIN_EXE_actus"))
            .args(["check", fixture.to_str().expect("rejected fixture path"), "--strict"])
            .current_dir(&root)
            .output()
            .expect("check rejected Phase 23.14 fixture");
        assert!(!rejected_check.status.success(), "fixture unexpectedly accepted: {fixture:?}");
    }
    let _ = std::fs::remove_file(binary);
    let _ = std::fs::remove_file(object);
}
