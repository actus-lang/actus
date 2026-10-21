use super::*;

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

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_non_niche_enum_aggregate_field_access() {
    let source = "struct Config { choice: Option[Int], } verb main() -> Int { erg config: Config = Config { choice: Option[Int].Some(41), }; return case config.choice { Option.Some(value) => value, Option.None => 0, }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-non-niche-enum-field-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create non-niche enum field directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"non-niche-enum-field-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write non-niche enum field manifest");
        fs::write(&input, source).expect("write non-niche enum field source");
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
        assert_eq!(result, 0, "backend {backend} should build enum aggregate field");
        let status =
            std::process::Command::new(&output).status().expect("run non-niche enum field fixture");
        assert_eq!(status.code(), Some(41), "backend {backend} enum aggregate result");
        let _ = fs::remove_dir_all(root);
    }
}

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

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_fixed_array_parameter_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-param-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb mutate(ins values: Array[u32, 4]) { values[2u32] = 41u32; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
    )
    .expect("write Cranelift fixed-array parameter store fixture");
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
        .expect("run Cranelift fixed-array parameter store fixture");
    assert_eq!(status.code(), Some(42));
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
fn cranelift_backend_builds_and_runs_a_shared_fixed_array_parameter_compound_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-param-compound-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb mutate(ins values: Array[u32, 4]) { values[2u32] += 41u32; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
    )
    .expect("write Cranelift fixed-array parameter compound fixture");
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
        .expect("run Cranelift fixed-array parameter compound fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_dynamic_fixed_array_parameter_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-param-dynamic-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb mutate(ins values: Array[u32, 4]) { erg index: u32 = 2u32; values[index] = 41u32; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
    )
    .expect("write Cranelift dynamic fixed-array parameter fixture");
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
        .expect("run Cranelift dynamic fixed-array parameter fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_array_struct_element_field_access() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-array-struct-field-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Point { x: u32, } verb mutate(ins values: Array[Point, 2]) { values[1u32].x = 41u32; } verb read(abs values: Array[Point, 2]) -> Int { return values[1u32].x as Int; } verb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); mutate(values: ins values); return read(values: abs values) + 1; }\n",
    )
    .expect("write Cranelift array-struct field fixture");
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
        .expect("run Cranelift array-struct field fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_aggregate_array_element_store() {
    let source = "struct Point { x: u32, } verb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); values[1u32] = Point { x: 41u32, }; return values[1u32].x as Int + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-aggregate-array-store-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create aggregate array store fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"aggregate-array-store-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write aggregate array store manifest");
        fs::write(&input, source).expect("write aggregate array store source");
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
        assert_eq!(result, 0, "backend {backend} should build aggregate array store");
        let status =
            std::process::Command::new(&output).status().expect("run aggregate array store");
        assert_eq!(status.code(), Some(42), "backend {backend} aggregate array result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_aggregate_array_element_replacement() {
    let source = "struct Point { x: u32, } verb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); values[0u32] = Point { x: 41u32, }; values[1u32] = values[0u32]; return values[1u32].x as Int + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-aggregate-array-replacement-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create aggregate replacement fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"aggregate-array-replacement-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write aggregate replacement manifest");
        fs::write(&input, source).expect("write aggregate replacement source");
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
        assert_eq!(result, 0, "backend {backend} should build aggregate replacement");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run aggregate replacement fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} should copy the aggregate element");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_nested_aggregate_array_element_store() {
    let source = "struct Point { x: u32, } struct Line { start: Point, } verb main() -> Int { erg values: Array[Line, 2] = Array[Line, 2](); values[1u32] = Line { start: Point { x: 41u32, }, }; return values[1u32].start.x as Int + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-nested-aggregate-array-store-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create nested aggregate array store directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"nested-aggregate-array-store-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write nested aggregate array store manifest");
        fs::write(&input, source).expect("write nested aggregate array store source");
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
        assert_eq!(result, 0, "backend {backend} should build nested aggregate store");
        let status =
            std::process::Command::new(&output).status().expect("run nested aggregate store");
        assert_eq!(status.code(), Some(42), "backend {backend} nested aggregate result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_generic_aggregate_array_element_store() {
    let source = "struct Box[T] { item: T, } verb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: 41, }; return boxes[1u32].item + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-generic-aggregate-array-store-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create generic aggregate array store directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"generic-aggregate-array-store-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write generic aggregate array store manifest");
        fs::write(&input, source).expect("write generic aggregate array store source");
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
        assert_eq!(result, 0, "backend {backend} should build generic aggregate store");
        let status =
            std::process::Command::new(&output).status().expect("run generic aggregate store");
        assert_eq!(status.code(), Some(42), "backend {backend} generic aggregate result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_discarded_aggregate_result_constructor() {
    let source = "enum Failure { Bad, } struct Counter { value: u32, } verb main() -> Int { Result[Counter, Failure].Ok(Counter { value: 41u32, }); return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-discarded-aggregate-result-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create discarded aggregate Result directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"discarded-aggregate-result-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write discarded aggregate Result manifest");
        fs::write(&input, source).expect("write discarded aggregate Result source");
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
        assert_eq!(result, 0, "backend {backend} should build discarded aggregate Result");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run discarded aggregate Result fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} aggregate Result result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_shared_discarded_option_constructors() {
    let source =
        "verb main() -> Int { Option[abs u32].Some(41u32); Option[abs u32].None; return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-discarded-option-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create discarded Option directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"discarded-option-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write discarded Option manifest");
        fs::write(&input, source).expect("write discarded Option source");
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
        assert_eq!(result, 0, "backend {backend} should build discarded Option");
        let status =
            std::process::Command::new(&output).status().expect("run discarded Option fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} discarded Option result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_share_aggregate_array_ins_caller_owned_storage() {
    let source = "struct Point { x: Int, } verb mutate(ins values: Array[Point, 2]) { values[1u32].x += 1; return; } verb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); values[1u32] = Point { x: 41, }; mutate(values: ins values); return values[1u32].x; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-aggregate-array-ins-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create aggregate array ins directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"aggregate-array-ins-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write aggregate array ins manifest");
        fs::write(&input, source).expect("write aggregate array ins source");
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
        assert_eq!(result, 0, "backend {backend} should build aggregate array ins fixture");
        let status =
            std::process::Command::new(&output).status().expect("run aggregate array ins fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} aggregate array ins result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_generic_aggregate_array_element_compound_store() {
    let source = "struct Box[T] { erg item: T, } verb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: 41, }; boxes[1u32].item += 1; return boxes[1u32].item; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-generic-aggregate-array-compound-store-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create generic aggregate compound directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"generic-aggregate-array-compound-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write generic aggregate compound manifest");
        fs::write(&input, source).expect("write generic aggregate compound source");
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
        assert_eq!(result, 0, "backend {backend} should build generic aggregate compound store");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run generic aggregate compound store");
        assert_eq!(status.code(), Some(42), "backend {backend} generic aggregate result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_nested_generic_aggregate_array_element_store() {
    let source = "struct Box[T] { item: T, } struct Pair[T] { item: T, } verb main() -> Int { erg pairs: Array[Pair[Box[Int]], 2] = Array[Pair[Box[Int]], 2](); pairs[1u32] = Pair[Box[Int]] { item: Box[Int] { item: 41, }, }; return pairs[1u32].item.item + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-nested-generic-aggregate-array-store-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create nested generic aggregate directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"nested-generic-aggregate-array-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write nested generic aggregate manifest");
        fs::write(&input, source).expect("write nested generic aggregate source");
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
        assert_eq!(result, 0, "backend {backend} should build nested generic aggregate");
        let status =
            std::process::Command::new(&output).status().expect("run nested generic aggregate");
        assert_eq!(status.code(), Some(42), "backend {backend} nested generic result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_generic_aggregate_option_view_field() {
    let source = "struct Box[T] { abs item: Option[abs T], } verb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: Option[abs Int].None, }; return case boxes[1u32].item { Option.Some(_) => 0, Option.None => 42, }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-generic-aggregate-option-view-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create generic option view directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"generic-aggregate-option-view-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write generic option view manifest");
        fs::write(&input, source).expect("write generic option view source");
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
        assert_eq!(result, 0, "backend {backend} should build generic option view");
        let status = std::process::Command::new(&output).status().expect("run generic option view");
        assert_eq!(status.code(), Some(42), "backend {backend} generic option view result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_generic_owned_buffer_field_transfer() {
    let source = "struct Box[T] { erg item: T, } verb consume(dat value: Buffer) { drop(value); return; } verb main() -> Int { erg boxed: Box[Buffer] = Box[Buffer] { item: Buffer[1], }; consume(value: dat boxed.item); return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-generic-owned-buffer-transfer-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create generic owned transfer directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"generic-owned-buffer-transfer-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write generic owned transfer manifest");
        fs::write(&input, source).expect("write generic owned transfer source");
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
        assert_eq!(result, 0, "backend {backend} should build generic owned transfer");
        let status =
            std::process::Command::new(&output).status().expect("run generic owned transfer");
        assert_eq!(status.code(), Some(42), "backend {backend} generic transfer result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_nested_generic_owned_buffer_field_transfer() {
    let source = "struct Box[T] { erg item: T, } struct Pair[T] { erg item: T, } verb consume(dat value: Buffer) { drop(value); return; } verb main() -> Int { erg pair: Pair[Box[Buffer]] = Pair[Box[Buffer]] { item: Box[Buffer] { item: Buffer[1], }, }; consume(value: dat pair.item.item); return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-nested-generic-owned-buffer-transfer-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create nested generic transfer directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"nested-generic-owned-buffer-transfer-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write nested generic transfer manifest");
        fs::write(&input, source).expect("write nested generic transfer source");
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
        assert_eq!(result, 0, "backend {backend} should build nested generic transfer");
        let status =
            std::process::Command::new(&output).status().expect("run nested generic transfer");
        assert_eq!(status.code(), Some(42), "backend {backend} nested transfer result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_fixed_array_parameter_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-llvm-array-param-store-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb mutate(ins values: Array[u32, 4]) { values[2u32] = 41u32; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
    )
    .expect("write LLVM fixed-array parameter store fixture");
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
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM fixed-array parameter store fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_fixed_array_parameter_compound_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-llvm-array-param-compound-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb mutate(ins values: Array[u32, 4]) { values[2u32] += 41u32; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
    )
    .expect("write LLVM fixed-array parameter compound fixture");
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
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM fixed-array parameter compound fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_dynamic_fixed_array_parameter_store() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-llvm-array-param-dynamic-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb mutate(ins values: Array[u32, 4]) { erg index: u32 = 2u32; values[index] = 41u32; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
    )
    .expect("write LLVM dynamic fixed-array parameter fixture");
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
    let status = std::process::Command::new(&output)
        .status()
        .expect("run LLVM dynamic fixed-array parameter fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_a_shared_array_struct_element_field_access() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-llvm-array-struct-field-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Point { x: u32, } verb mutate(ins values: Array[Point, 2]) { values[1u32].x = 41u32; } verb read(abs values: Array[Point, 2]) -> Int { return values[1u32].x as Int; } verb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); mutate(values: ins values); return read(values: abs values) + 1; }\n",
    )
    .expect("write LLVM array-struct field fixture");
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
    let status =
        std::process::Command::new(&output).status().expect("run LLVM array-struct field fixture");
    assert_eq!(status.code(), Some(42));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
