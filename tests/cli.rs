use std::fs;
#[cfg(unix)]
use std::process::Command;

use actus::cli::run_with_args;

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_scalar_root_program() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create LLVM fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM fixture manifest");
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write LLVM fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM scalar fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_array_pack_bit_fields() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-pack-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create LLVM pack fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-pack-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM pack fixture manifest");
    fs::write(
        &input,
        "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } verb main() -> Int { erg registers: Array[Register, 2] = Array[Register, 2](); registers[1] = Register { storage: 0, }; registers[1].enabled = 1; return registers[1].enabled; }\n",
    )
    .expect("write LLVM pack fixture source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM pack fixture");
    assert_eq!(status.code(), Some(1));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_phase18_arrays_and_packs_example() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-phase18-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create LLVM Phase 18 fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-phase18-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM Phase 18 fixture manifest");
    let source = format!("{}/examples/phase18_arrays_and_packs.act", env!("CARGO_MANIFEST_DIR"));
    fs::copy(source, &input).expect("copy Phase 18 source fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--strict".to_owned(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM Phase 18 fixture");
    assert_eq!(status.code(), Some(92));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_user_defined_drop_performance() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-drop-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create LLVM Drop fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-drop-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM Drop fixture manifest");
    fs::write(
        &input,
        "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { return; } } verb main() -> Int { erg counter: Counter = Counter { value: 7, }; return 42; }\n",
    )
    .expect("write LLVM Drop fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM Drop fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_scalar_result_program() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-result-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create LLVM Result fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-result-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM Result fixture manifest");
    fs::write(
        &input,
        "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb main() -> Int { erg result = produce(); return case dat result { Result.Ok(value) => value + 1, Result.Err(_) => 0, }; }\n",
    )
    .expect("write LLVM Result fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM Result fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn build_backend_override_uses_the_same_validated_llvm_configuration() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-override-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create LLVM override fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-cli-override\"\nversion = \"1.0.0\"\n",
    )
    .expect("write LLVM override fixture manifest");
    fs::write(&input, "verb main() -> Int { return 43; }\n")
        .expect("write LLVM override fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--backend".to_owned(),
            "llvm".to_owned(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM override fixture");
    assert_eq!(status.code(), Some(43));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn run_backend_override_uses_the_same_validated_llvm_configuration() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-run-llvm-override-{}", std::process::id()));
    let input = root.join("main.act");
    fs::create_dir_all(&root).expect("create LLVM run override fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-run-cli-override\"\nversion = \"1.0.0\"\n",
    )
    .expect("write LLVM run override fixture manifest");
    fs::write(&input, "verb main() -> Int { return 44; }\n")
        .expect("write LLVM run override fixture source");

    let result = run_with_args(
        vec![
            "run".to_owned(),
            input.display().to_string(),
            "--backend".to_owned(),
            "llvm".to_owned(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 44);
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_scalar_imported_module() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-module-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src/math")).expect("create LLVM module fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-module-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM module fixture manifest");
    fs::write(&input, "import math; verb main() -> Int { return add(); }\n")
        .expect("write LLVM module root source");
    fs::write(root.join("src/math/math.act"), "open api;\n").expect("write LLVM module facade");
    fs::write(root.join("src/math/api.act"), "open verb add() -> Int { return 40; }\n")
        .expect("write LLVM module implementation");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM imported module fixture");
    assert_eq!(status.code(), Some(40));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm", feature = "cranelift"))]
#[test]
fn llvm_backend_builds_and_runs_a_scalar_const_generic() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-generic-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create LLVM generic fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-generic-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM generic fixture manifest");
    fs::write(
        &input,
        "verb read_capacity[N: Usize]() -> u32 { return N as u32; } verb main() -> u32 { return read_capacity[4](); }\n",
    )
    .expect("write LLVM generic fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM generic fixture");
    assert_eq!(status.code(), Some(4));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm", feature = "cranelift"))]
#[test]
fn llvm_backend_builds_and_runs_a_const_generic_from_an_imported_module() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-module-generic-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src/math"))
        .expect("create LLVM generic module fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-module-generic-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM generic module fixture manifest");
    fs::write(&input, "import math; verb main() -> u32 { return read_capacity[4](); }\n")
        .expect("write LLVM generic module root source");
    fs::write(root.join("src/math/math.act"), "open api;\n")
        .expect("write LLVM generic module facade");
    fs::write(
        root.join("src/math/api.act"),
        "open verb read_capacity[N: Usize]() -> u32 { return N as u32; }\n",
    )
    .expect("write LLVM generic module implementation");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM generic module fixture");
    assert_eq!(status.code(), Some(4));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_scalar_const_generic_through_shared_lowering() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-generic-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create Cranelift generic fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"cranelift-generic-cli\"\nversion = \"1.0.0\"\n",
    )
    .expect("write Cranelift generic fixture manifest");
    fs::write(
        &input,
        "verb read_capacity[N: Usize]() -> u32 { return N as u32; } verb main() -> u32 { return read_capacity[4](); }\n",
    )
    .expect("write Cranelift generic fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run Cranelift generic fixture");
    assert_eq!(status.code(), Some(4));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_scalar_imported_module_through_shared_lowering() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-module-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src/math")).expect("create Cranelift module fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"cranelift-module-cli\"\nversion = \"1.0.0\"\n",
    )
    .expect("write Cranelift module fixture manifest");
    fs::write(&input, "import math; verb main() -> Int { return add(); }\n")
        .expect("write Cranelift module root source");
    fs::write(root.join("src/math/math.act"), "open api;\n")
        .expect("write Cranelift module facade");
    fs::write(root.join("src/math/api.act"), "open verb add() -> Int { return 40; }\n")
        .expect("write Cranelift module implementation");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift imported module fixture");
    assert_eq!(status.code(), Some(40));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_scalar_if_through_shared_lowering() {
    let root = std::env::temp_dir().join(format!("actus-cli-cranelift-if-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { if true { return 42; } else { return 7; } }\n")
        .expect("write Cranelift shared if fixture");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run Cranelift shared if fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_scalar_result_through_shared_lowering() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-result-{}", std::process::id()));
    let input = root.join("main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(&root).expect("create Cranelift Result fixture directory");
    fs::write(
        &input,
        "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb main() -> Int { erg result = produce(); return case dat result { Result.Ok(value) => value + 1, Result.Err(_) => 0, }; }\n",
    )
    .expect("write Cranelift Result fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run Cranelift Result fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_scalar_assignment_through_shared_lowering() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-assignment-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { erg value = 40; value += 2; return value; }\n")
        .expect("write Cranelift shared assignment fixture");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift shared assignment fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_scalar_aggregate_field_access() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-field-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; return counter.value + 1; }\n",
    )
    .expect("write Cranelift aggregate field fixture");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift aggregate field fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_scalar_aggregate_field_assignment() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-field-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value = 42; return counter.value; }\n",
    )
    .expect("write Cranelift aggregate field assignment fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift aggregate field assignment fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_scalar_aggregate_field_compound_assignment() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-field-compound-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value += 1; return counter.value; }\n",
    )
    .expect("write Cranelift aggregate field compound fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift aggregate field compound fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_checked_scalar_division_remainder_and_shifts() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-checked-integer-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { return (84 / 2) + ((1 << 3) - 8) + ((5 % 2) - 1); }\n")
        .expect("write Cranelift checked integer fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift checked integer fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_scalar_loop() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-loop-shared-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value: Int = 41; loop { value += 1; break; } return value; }\n",
    )
    .expect("write Cranelift shared loop fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run Cranelift shared loop fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_range_loop() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-range-loop-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 0u32 .. 4u32 { total += index; } return total as Int; }\n",
    )
    .expect("write Cranelift shared range loop fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift shared range loop fixture");
    assert_eq!(status.code(), Some(6));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_fixed_array_loop() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-fixed-array-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); erg total: u32 = 0u32; for erg index: u32 in values { total += values[index]; } return total as Int; }\n",
    )
    .expect("write Cranelift shared fixed-array fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift shared fixed-array fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_shared_scalar_control_flow() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-control-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src")).expect("create LLVM control-flow fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-control-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM control-flow fixture manifest");
    fs::write(
        &input,
        "verb main() -> Int { erg ready = 0; ready = 1; if ready == 1 { return 42; } else { return 7; }; }\n",
    )
    .expect("write LLVM control-flow fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM control-flow fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_shared_print_runtime_bridge() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-print-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src")).expect("create LLVM print fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-print-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM print fixture manifest");
    fs::write(&input, "verb main() -> Int { print(42); return 42; }\n")
        .expect("write LLVM print fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM print fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_scalar_assignment_through_shared_lowering() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-assignment-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { erg value = 40; value += 2; return value; }\n")
        .expect("write LLVM shared assignment fixture");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM shared assignment fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_range_loop() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-range-loop-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src")).expect("create LLVM range loop fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-range-loop-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM range loop fixture manifest");
    fs::write(
        &input,
        "verb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 0u32 .. 4u32 { total += index; } return total as Int; }\n",
    )
    .expect("write LLVM shared range loop fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM shared range loop fixture");
    assert_eq!(status.code(), Some(6));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_fixed_array_loop() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-fixed-array-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src")).expect("create LLVM fixed-array fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-fixed-array-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM fixed-array fixture manifest");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); erg total: u32 = 0u32; for erg index: u32 in values { total += values[index]; } return total as Int; }\n",
    )
    .expect("write LLVM shared fixed-array fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM shared fixed-array fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_fixed_array_index_read() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-index-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[2u32] as Int; }\n",
    )
    .expect("write Cranelift direct array index fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift direct array index fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_fixed_array_index_read() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-array-index-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[2u32] as Int; }\n",
    )
    .expect("write LLVM direct array index fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM direct array index fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_fixed_array_parameter_read() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-param-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb read(abs values: Array[u32, 4]) -> Int { return values[2u32] as Int; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return read(values: abs values); }\n",
    )
    .expect("write Cranelift fixed-array parameter fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift fixed-array parameter fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_fixed_array_parameter_read() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-array-param-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb read(abs values: Array[u32, 4]) -> Int { return values[2u32] as Int; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return read(values: abs values); }\n",
    )
    .expect("write LLVM fixed-array parameter fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM fixed-array parameter fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_scalar_struct_parameter_read() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-struct-param-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb read(abs counter: Counter) -> Int { return counter.value; } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; return read(counter: abs counter) + 1; }\n",
    )
    .expect("write Cranelift scalar-struct parameter fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift scalar-struct parameter fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_scalar_struct_parameter_read() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-struct-param-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb read(abs counter: Counter) -> Int { return counter.value; } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; return read(counter: abs counter) + 1; }\n",
    )
    .expect("write LLVM scalar-struct parameter fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM scalar-struct parameter fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_fixed_array_element_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] = 41u32; return values[2u32] as Int + 1; }\n",
    )
    .expect("write Cranelift fixed-array store fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift fixed-array store fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_fixed_array_element_store() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-array-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] = 41u32; return values[2u32] as Int + 1; }\n",
    )
    .expect("write LLVM fixed-array store fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM fixed-array store fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_fixed_array_element_compound_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-compound-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] += 41u32; return values[2u32] as Int + 1; }\n",
    )
    .expect("write Cranelift fixed-array compound store fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "cranelift",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is UTF-8"),
            input.to_str().expect("input path is UTF-8"),
        ])
        .status()
        .expect("build Cranelift fixed-array compound store fixture");
    assert!(status.success());
    let status =
        Command::new(&output).status().expect("run Cranelift fixed-array compound store fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_fixed_array_element_compound_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-llvm-array-compound-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] += 41u32; return values[2u32] as Int + 1; }\n",
    )
    .expect("write LLVM fixed-array compound store fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "llvm",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is UTF-8"),
            input.to_str().expect("input path is UTF-8"),
        ])
        .status()
        .expect("build LLVM fixed-array compound store fixture");
    assert!(status.success());
    let status =
        Command::new(&output).status().expect("run LLVM fixed-array compound store fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_dynamic_fixed_array_index() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-dynamic-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[index] += 41u32; return values[index] as Int + 1; } verb main() -> Int { return read(index: abs 2u32); }\n",
    )
    .expect("write Cranelift dynamic fixed-array fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "cranelift",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is UTF-8"),
            input.to_str().expect("input path is UTF-8"),
        ])
        .status()
        .expect("build Cranelift dynamic fixed-array fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run Cranelift dynamic fixed-array fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_dynamic_fixed_array_index() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-array-dynamic-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[index] += 41u32; return values[index] as Int + 1; } verb main() -> Int { return read(index: abs 2u32); }\n",
    )
    .expect("write LLVM dynamic fixed-array fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "llvm",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is UTF-8"),
            input.to_str().expect("input path is UTF-8"),
        ])
        .status()
        .expect("build LLVM dynamic fixed-array fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run LLVM dynamic fixed-array fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_traps_on_a_shared_dynamic_fixed_array_index_out_of_bounds() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-array-oob-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[index] as Int; } verb main() -> Int { return read(index: abs 4u32); }\n",
    )
    .expect("write Cranelift array bounds fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "cranelift",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is UTF-8"),
            input.to_str().expect("input path is UTF-8"),
        ])
        .status()
        .expect("build Cranelift array bounds fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run Cranelift array bounds fixture");
    assert!(!status.success());
    assert!(status.code().is_none(), "array bounds failure should be a native trap");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_traps_on_a_shared_dynamic_fixed_array_index_out_of_bounds() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-array-oob-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[index] as Int; } verb main() -> Int { return read(index: abs 4u32); }\n",
    )
    .expect("write LLVM array bounds fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "llvm",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is UTF-8"),
            input.to_str().expect("input path is UTF-8"),
        ])
        .status()
        .expect("build LLVM array bounds fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run LLVM array bounds fixture");
    assert!(!status.success());
    assert!(status.code().is_none(), "array bounds failure should be a native trap");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_scalar_struct_parameter_mutation() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-struct-mutation-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb increment(ins counter: Counter) { counter.value += 1; return; } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; increment(counter: ins counter); return counter.value; }\n",
    )
    .expect("write Cranelift scalar-struct mutation fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run Cranelift scalar-struct mutation fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_scalar_struct_parameter_mutation() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-struct-mutation-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb increment(ins counter: Counter) { counter.value += 1; return; } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; increment(counter: ins counter); return counter.value; }\n",
    )
    .expect("write LLVM scalar-struct mutation fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM scalar-struct mutation fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_scalar_aggregate_field_access() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-field-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; return counter.value + 1; }\n",
    )
    .expect("write LLVM aggregate field fixture");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run LLVM aggregate field fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_scalar_aggregate_field_assignment() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-field-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value = 42; return counter.value; }\n",
    )
    .expect("write LLVM aggregate field assignment fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM aggregate field assignment fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_scalar_aggregate_field_compound_assignment() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-field-compound-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value += 1; return counter.value; }\n",
    )
    .expect("write LLVM aggregate field compound fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM aggregate field compound fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_shared_scalar_bitwise_operations() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-bitwise-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src")).expect("create LLVM bitwise fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-bitwise-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM bitwise fixture manifest");
    fs::write(&input, "verb main() -> Int { return (12 & 10) | 1; }\n")
        .expect("write LLVM bitwise fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM bitwise fixture");
    assert_eq!(status.code(), Some(9));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_checked_scalar_division_and_remainder() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-division-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src")).expect("create LLVM division fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-division-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM division fixture manifest");
    fs::write(&input, "verb main() -> Int { return 20 / 5 + 17 % 6; }\n")
        .expect("write LLVM division fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM division fixture");
    assert_eq!(status.code(), Some(9));
    let _ = fs::remove_dir_all(root);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_checked_scalar_shifts() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-shift-{}", std::process::id()));
    let input = root.join("src/main.act");
    let output = root.join("main.bin");
    fs::create_dir_all(root.join("src")).expect("create LLVM shift fixture directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"llvm-shift-cli\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM shift fixture manifest");
    fs::write(&input, "verb main() -> Int { return (1 << 4) + (32 >> 3); }\n")
        .expect("write LLVM shift fixture source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run LLVM shift fixture");
    assert_eq!(status.code(), Some(20));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn build_command_writes_a_native_object() {
    let root = std::env::temp_dir().join(format!("actus-cli-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("o");
    fs::write(
        &input,
        "verb main() -> Int { erg left = 40; erg right = 2; return add(left: left, right: right); } verb add(erg left: Int, erg right: Int) -> Int { return left + right; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let object = fs::read(&output).expect("read object");
    object::File::parse(object.as_slice()).expect("output should be a native object");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn run_command_builds_and_executes_source() {
    let input = std::env::temp_dir().join(format!("actus-cli-run-{}.act", std::process::id()));
    fs::write(&input, "verb main() -> Int { return 42; }\n").expect("write source");

    let result = run_with_args(vec!["run".to_owned(), input.display().to_string()].into_iter());

    assert_eq!(result, 42);
    let _ = fs::remove_file(input);
}

#[cfg(unix)]
#[test]
fn build_command_returns_zero_for_a_void_hosted_entry() {
    let root = std::env::temp_dir().join(format!("actus-cli-void-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Void { return; }\n").expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = Command::new(&output).status().expect("run Void executable");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_links_an_executable() {
    let root = std::env::temp_dir().join(format!("actus-cli-exe-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg left = 40; erg right = 2; return add(left: left, right: right); } verb add(erg left: Int, erg right: Int) -> Int { return left + right; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "--release".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    assert!(!output.with_extension("o").exists());
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_unary_and_grouped_expression() {
    let root = std::env::temp_dir().join(format!("actus-cli-expression-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { return -(2 + 3) * -4; }\n").expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(20));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_nested_scalar_scope_and_drop() {
    let root = std::env::temp_dir().join(format!("actus-cli-scope-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 41; { abs view = ref value; } drop(value); return 42; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_unwinds_borrow_scope_on_early_return() {
    let root = std::env::temp_dir().join(format!("actus-cli-borrow-return-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 41; { abs view = ref value; return view + 1; } }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_scalar_dat_transfer_without_duplicate_cleanup() {
    let root = std::env::temp_dir().join(format!("actus-cli-dat-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 41; return consume(value: value); } verb consume(dat value: Int) -> Int { return value + 1; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_loop_break_control_flow() {
    let root = std::env::temp_dir().join(format!("actus-cli-loop-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { loop { break; } return 42; }\n").expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_resolves_break_inside_case_to_the_enclosing_loop() {
    let root = std::env::temp_dir().join(format!("actus-cli-case-loop-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg flag = 1; loop { case flag { 1 => { break; }, _ => { continue; }, }; } return 42; }\n",
    )
    .expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_resolves_return_inside_case_to_the_enclosing_verb() {
    let root = std::env::temp_dir().join(format!("actus-cli-case-return-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { case 1 { 1 => { return 42; }, _ => { return 0; }, }; return 1; }\n",
    )
    .expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_resolves_continue_inside_case_to_the_enclosing_loop() {
    let root = std::env::temp_dir().join(format!("actus-cli-case-continue-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg flag = 1; loop { case flag { 1 => { flag += 1; continue; }, _ => { break; }, }; } return flag; }\n",
    )
    .expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(2));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_unsigned_relational_comparison() {
    let root = std::env::temp_dir().join(format!("actus-cli-relational-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg left: u8 = 7; erg right: u8 = 7; return case left >= right { true => 0, _ => 1, }; }\n",
    )
    .expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_if_else_control_flow() {
    let root = std::env::temp_dir().join(format!("actus-cli-if-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg ready = 0; if ready == 1 { print(1); } else { print(42); }; return 0; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_statement_if_break_inside_a_loop() {
    let root = std::env::temp_dir().join(format!("actus-cli-if-break-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { loop { if 1 == 1 { break; } } return 42; }\n")
        .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_signed_relational_comparison() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-signed-relational-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg left: i8 = -3; erg right: i8 = 2; return case left < right { true => 0, _ => 1, }; }\n",
    )
    .expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_float_relational_comparison() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-float-relational-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg left: f32 = 1.5; erg right: f32 = 2.5; return case left <= right { true => 0, _ => 1, }; }\n",
    )
    .expect("write source");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_executes_return_from_loop_body() {
    let root = std::env::temp_dir().join(format!("actus-cli-loop-return-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, "verb main() -> Int { loop { return 42; } }\n").expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_preserves_loop_carried_binding_values() {
    let root = std::env::temp_dir().join(format!("actus-cli-loop-ssa-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg value = 0; loop { value = 1; break; } return value; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(1));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn build_command_links_external_c_symbols() {
    let root = std::env::temp_dir().join(format!("actus-cli-external-c-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "unsafe extern \"C\" verb rand() -> Int; verb main() -> Int { rand(); return 42; }\n",
    )
    .expect("write source");

    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );

    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn check_command_rejects_semantically_invalid_source() {
    let input = std::env::temp_dir().join(format!("actus-cli-check-{}.act", std::process::id()));
    fs::write(&input, "verb main() { inspect(missing); }\n").expect("write source");

    let result = run_with_args(vec!["check".to_owned(), input.display().to_string()].into_iter());

    assert_eq!(result, 1);
    let _ = fs::remove_file(input);
}

#[test]
fn top_level_help_flags_succeed() {
    assert_eq!(run_with_args(vec!["--help".to_owned()].into_iter()), 0);
    assert_eq!(run_with_args(vec!["-h".to_owned()].into_iter()), 0);
}

#[test]
fn build_help_does_not_require_an_input_file() {
    let result = run_with_args(vec!["build".to_owned(), "--help".to_owned()].into_iter());

    assert_eq!(result, 0);
}

#[test]
fn unknown_command_help_returns_an_error() {
    let result = run_with_args(vec!["unknown".to_owned(), "--help".to_owned()].into_iter());

    assert_eq!(result, 2);
}
