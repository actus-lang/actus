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
