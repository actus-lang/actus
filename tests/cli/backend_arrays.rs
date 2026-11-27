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
        "struct Counter { value: Int, }\n\"\"\"Entry point that returns the value of a Counter holding 41 plus 1.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; return counter.value + 1; }\n",
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
    let source = "struct Config { choice: Option[Int], }\n\"\"\"Entry point that builds a Config with the choice Some(41) and returns the payload, or 0 when it is None.\"\"\"\nverb main() -> Int { erg config: Config = Config { choice: Option[Int].Some(41), }; return case config.choice { Option.Some(value) => value, Option.None => 0, }; }\n";
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

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_preserve_aggregate_enum_payload_assignment_to_exclusive_loan() {
    let source = "\"\"\"\ncontract:\npurpose:\n    Replace the destination buffer with the buffer carried by an Option and return its first byte, or 1 when the Option is None.\ninputs:\n    destination: exclusive ins loan of Buffer that is overwritten; selected: owned Option of Buffer.\noutputs:\n    The first byte of the new destination as Int, or 1 for None.\nownership:\n    The destination is an exclusive ins loan that is replaced; the selected Option is consumed and its buffer is moved into the destination.\nside_effects:\n    Replaces the destination buffer.\n\"\"\"\nverb copy_payload(ins destination: Buffer, dat selected: Option[Buffer]) -> Int { return case dat selected { Option.Some(value) => { destination = value; return destination[0] as Int; }, Option.None => 1, }; }\n\"\"\"Entry point that wraps a one-byte buffer holding 42 in Some, passes it to copy_payload with an empty destination and returns the result.\"\"\"\nverb main() -> Int { erg source: Buffer = Buffer[0]; append(source, 42u8); erg selected = Option[Buffer].Some(dat source); erg destination: Buffer = Buffer[0]; return copy_payload(destination: ins destination, selected: dat selected); }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-enum-loan-assignment-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create enum loan assignment fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"enum-loan-assignment-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write enum loan assignment fixture manifest");
        fs::write(&input, source).expect("write enum loan assignment fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build enum loan assignment");
        let status =
            std::process::Command::new(&output).status().expect("run enum loan assignment fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} enum loan assignment result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_preserve_direct_buffer_assignment_to_exclusive_loan() {
    let source = "\"\"\"\ncontract:\npurpose:\n    Replace the destination buffer with the source buffer and return its first byte.\ninputs:\n    destination: exclusive ins loan of Buffer that is overwritten; source: owned Buffer that is moved into the destination.\noutputs:\n    The first byte of the new destination as Int.\nownership:\n    The destination is an exclusive ins loan that is replaced; the source is consumed.\nside_effects:\n    Replaces the destination buffer.\n\"\"\"\nverb copy_payload(ins destination: Buffer, dat source: Buffer) -> Int { destination = source; return destination[0] as Int; }\n\"\"\"Entry point that builds a one-byte buffer holding 42, moves it into an empty destination through copy_payload and returns the first byte.\"\"\"\nverb main() -> Int { erg source: Buffer = Buffer[0]; append(source, 42u8); erg destination: Buffer = Buffer[0]; return copy_payload(destination: ins destination, source: dat source); }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-direct-loan-assignment-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create direct loan assignment fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"direct-loan-assignment-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write direct loan assignment manifest");
        fs::write(&input, source).expect("write direct loan assignment source");
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
        assert_eq!(result, 0, "backend {backend} should build direct loan assignment");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run direct loan assignment fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} direct loan result");
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
        "struct Counter { value: Int, }\n\"\"\"Entry point that assigns 42 to the value of a Counter holding 41 and returns it.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value = 42; return counter.value; }\n",
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
        "struct Counter { value: Int, }\n\"\"\"Entry point that adds 1 to the value of a Counter holding 41 with a compound assignment and returns it.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value += 1; return counter.value; }\n",
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
    fs::write(&input, "\"\"\"Entry point that returns the sum of 84 divided by 2, 1 shifted left by 3 minus 8, and 5 modulo 2 minus 1.\"\"\"\nverb main() -> Int { return (84 / 2) + ((1 << 3) - 8) + ((5 % 2) - 1); }\n")
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
        "\"\"\"Entry point that adds 1 to 41 inside a loop that breaks at once and returns the value.\"\"\"\nverb main() -> Int { erg value: Int = 41; loop { value += 1; break; } return value; }\n",
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
        "\"\"\"Entry point that sums the indices 0 through 3 of a counted for loop into a u32 and returns the total as Int.\"\"\"\nverb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 0u32 .. 4u32 { total += index; } return total as Int; }\n",
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
        "\"\"\"Entry point that iterates a for loop over the index space of a four-element u32 array, adds each element to a total and returns it as Int.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); erg total: u32 = 0u32; for erg index: u32 in values { total += values[index]; } return total as Int; }\n",
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
        "\"\"\"Entry point that sets ready to 1 and returns 42 when it equals 1, otherwise 7.\"\"\"\nverb main() -> Int { erg ready = 0; ready = 1; if ready == 1 { return 42; } else { return 7; }; }\n",
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
    fs::write(&input, "\"\"\"\ncontract:\npurpose:\n    Print the number 42 and return it as the exit status.\noutputs:\n    42.\nside_effects:\n    Writes the number 42 to standard output.\n\"\"\"\nverb main() -> Int { print(42); return 42; }\n")
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
    fs::write(&input, "\"\"\"Entry point that adds 2 to 40 with a compound assignment and returns the result.\"\"\"\nverb main() -> Int { erg value = 40; value += 2; return value; }\n")
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
        "\"\"\"Entry point that sums the indices 0 through 3 of a counted for loop into a u32 and returns the total as Int.\"\"\"\nverb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 0u32 .. 4u32 { total += index; } return total as Int; }\n",
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
        "\"\"\"Entry point that iterates a for loop over the index space of a four-element u32 array, adds each element to a total and returns it as Int.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); erg total: u32 = 0u32; for erg index: u32 in values { total += values[index]; } return total as Int; }\n",
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
        "\"\"\"Entry point that returns element 2 of a freshly zeroed u32 array, as Int.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[2u32] as Int; }\n",
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
        "\"\"\"Entry point that returns element 2 of a freshly zeroed u32 array, as Int.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[2u32] as Int; }\n",
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
        "\"\"\"Returns element 2 of a borrowed four-element u32 array as Int.\"\"\"\nverb read(abs values: Array[u32, 4]) -> Int { return values[2u32] as Int; }\n\"\"\"Entry point that passes a zeroed u32 array to read as a borrow and returns the result.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return read(values: abs values); }\n",
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
        "\"\"\"Stores 41 in element 2 of an exclusively borrowed four-element u32 array.\"\"\"\nverb mutate(ins values: Array[u32, 4]) { values[2u32] = 41u32; }\n\"\"\"Entry point that fills element 2 of a u32 array through mutate and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
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
        "\"\"\"Returns element 2 of a borrowed four-element u32 array as Int.\"\"\"\nverb read(abs values: Array[u32, 4]) -> Int { return values[2u32] as Int; }\n\"\"\"Entry point that passes a zeroed u32 array to read as a borrow and returns the result.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return read(values: abs values); }\n",
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
        "\"\"\"Adds 41 to element 2 of an exclusively borrowed four-element u32 array with a compound assignment.\"\"\"\nverb mutate(ins values: Array[u32, 4]) { values[2u32] += 41u32; }\n\"\"\"Entry point that fills element 2 of a u32 array through mutate and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
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
        "\"\"\"Stores 41 in the element of an exclusively borrowed u32 array selected by the local index 2.\"\"\"\nverb mutate(ins values: Array[u32, 4]) { erg index: u32 = 2u32; values[index] = 41u32; }\n\"\"\"Entry point that fills element 2 of a u32 array through mutate and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
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
        "struct Point { x: u32, }\n\"\"\"Stores 41 in the x field of element 1 of an exclusively borrowed array of Point structs.\"\"\"\nverb mutate(ins values: Array[Point, 2]) { values[1u32].x = 41u32; }\n\"\"\"Returns the x field of element 1 of a borrowed array of Point structs, as Int.\"\"\"\nverb read(abs values: Array[Point, 2]) -> Int { return values[1u32].x as Int; }\n\"\"\"Entry point that sets the x field of element 1 through mutate, reads it through read and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); mutate(values: ins values); return read(values: abs values) + 1; }\n",
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
    let source = "struct Point { x: u32, }\n\"\"\"Entry point that assigns a Point with x 41 to element 1 of an array and returns its x plus 1.\"\"\"\nverb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); values[1u32] = Point { x: 41u32, }; return values[1u32].x as Int + 1; }\n";
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
    let source = "struct Point { x: u32, }\n\"\"\"Entry point that stores a Point with x 41 in element 0, copies it to element 1 and returns the x of element 1 plus 1.\"\"\"\nverb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); values[0u32] = Point { x: 41u32, }; values[1u32] = values[0u32]; return values[1u32].x as Int + 1; }\n";
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
fn both_native_backends_write_through_a_dynamic_nested_aggregate_index() {
    let source = "struct Record { targets: Array[u32, 2], } struct State { records: Array[Record, 2], }\n\"\"\"Stores 41 in the first target of an exclusively borrowed Record.\"\"\"\nverb initialize(ins record: Record) { record.targets[0u32] = 41u32; }\n\"\"\"Entry point that initializes the record at index 1 of a State and returns its first target plus 1.\"\"\"\nverb main() -> Int { erg state: State = State { records: Array[Record, 2](), }; erg index: u32 = 1u32; initialize(record: ins state.records[index]); return state.records[index].targets[0u32] as Int + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-dynamic-nested-aggregate-index-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create dynamic nested aggregate fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"dynamic-nested-aggregate-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write dynamic nested aggregate manifest");
        fs::write(&input, source).expect("write dynamic nested aggregate source");
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
        assert_eq!(result, 0, "backend {backend} should build dynamic nested aggregate");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run dynamic nested aggregate fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} dynamic nested aggregate result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_initialize_and_mutate_a_three_level_dynamic_aggregate() {
    let source = "struct Record { targets: Array[u32, 2], } struct Table { records: Array[Record, 2], active_count: u32, } struct State { table: Table, } struct Brain { auxiliary: Array[State, 2], }\n\"\"\"Marks the first target of the first record in the state table as the 4294967295 sentinel and clears the active count.\"\"\"\nverb initialize(ins state: State) { state.table.records[0u32].targets[0u32] = 4294967295u32; state.table.active_count = 0u32; }\n\"\"\"Points the first target of the first record at record 0 and increments the active count of the state table.\"\"\"\nverb link(ins state: State) { state.table.records[0u32].targets[0u32] = 0u32; state.table.active_count += 1u32; }\n\"\"\"Entry point that initializes two State values of a Brain, links the second one, and returns 42 when the first keeps the sentinel and the second holds the link, otherwise 43 or 44.\"\"\"\nverb main() -> Int { erg brain: Brain = Brain { auxiliary: Array[State, 2](), }; erg index: u32 = 0u32; initialize(state: ins brain.auxiliary[index]); index = 1u32; initialize(state: ins brain.auxiliary[index]); link(state: ins brain.auxiliary[index]); return if brain.auxiliary[0u32].table.records[0u32].targets[0u32] == 4294967295u32 { if brain.auxiliary[1u32].table.records[0u32].targets[0u32] == 0u32 { 42 } else { 43 } } else { 44 }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-three-level-dynamic-aggregate-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create three-level dynamic aggregate fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"three-level-dynamic-aggregate-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write three-level dynamic aggregate manifest");
        fs::write(&input, source).expect("write three-level dynamic aggregate source");
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
        assert_eq!(result, 0, "backend {backend} should build three-level dynamic aggregate");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run three-level dynamic aggregate fixture");
        assert_eq!(
            status.code(),
            Some(42),
            "backend {backend} three-level dynamic aggregate result"
        );
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_write_through_large_dynamic_nested_aggregate_state() {
    let source = "struct Record { targets: Array[u32, 8], used_count: u8, reserved: Array[u8, 3], free_list_link: u32, } struct Table { source_to_record: Array[u16, 1024], records: Array[Record, 8], free_list_head: u32, active_count: u32, } struct State { table: Table, } struct Brain { auxiliary: Array[State, 2], }\n\"\"\"Resets a State table: it sets every source mapping to the 65535 none marker, every record target to the 4294967295 sentinel, and the free list head and active count to 0.\"\"\"\nverb initialize(ins state: State) { for erg source: u32 in 0u32 .. 1024u32 { state.table.source_to_record[source] = 65535u16; } for erg record: u32 in 0u32 .. 8u32 { for erg target: u32 in 0u32 .. 8u32 { state.table.records[record].targets[target] = 4294967295u32; } } state.table.free_list_head = 0u32; state.table.active_count = 0u32; }\n\"\"\"Maps a source index to record 0, stores the source as its first target, increments the active count and returns the new count as Int.\"\"\"\nverb link(ins state: State, erg source: u32) -> Int { state.table.source_to_record[source] = 0u16; state.table.records[0u32].targets[0u32] = source; state.table.active_count += 1u32; return state.table.active_count as Int; }\n\"\"\"Entry point that initializes two State values of a Brain, links source 1023 in the second one, and returns the link count plus 41 when the mapping and target are correct, otherwise 43 or 44.\"\"\"\nverb main() -> Int { erg brain: Brain = Brain { auxiliary: Array[State, 2](), }; erg index: u32 = 0u32; initialize(state: ins brain.auxiliary[index]); index = 1u32; initialize(state: ins brain.auxiliary[index]); erg source_index: u32 = 1023u32; erg link_status: Int = link(state: ins brain.auxiliary[index], source: erg source_index); return if brain.auxiliary[1u32].table.source_to_record[1023u32] == 0u16 { if brain.auxiliary[1u32].table.records[0u32].targets[0u32] == 1023u32 { link_status + 41 } else { 43 } } else { 44 }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-large-dynamic-nested-state-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create large dynamic nested state fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"large-dynamic-nested-state-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write large dynamic nested state manifest");
        fs::write(&input, source).expect("write large dynamic nested state source");
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
        assert_eq!(result, 0, "backend {backend} should build large dynamic nested state");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run large dynamic nested state fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} large dynamic nested state result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_round_trip_a_large_nested_aggregate_snapshot() {
    let source = "struct Record { targets: Array[u32, 8], used_count: u8, reserved: Array[u8, 3], free_list_link: u32, } struct Table { source_to_record: Array[u16, 1024], records: Array[Record, 8], free_list_head: u32, active_count: u32, } struct State { table: Table, } struct Snapshot { table: Table, checksum: u64, } struct Brain { prefix: Array[u8, 524288], auxiliary: Array[State, 2], }\n\"\"\"Resets a State table: it sets every source mapping to the 65535 none marker, every record target to the 4294967295 sentinel, and the free list head and active count to 0.\"\"\"\nverb initialize(ins state: State) { for erg source: u32 in 0u32 .. 1024u32 { state.table.source_to_record[source] = 65535u16; } for erg record: u32 in 0u32 .. 8u32 { for erg target: u32 in 0u32 .. 8u32 { state.table.records[record].targets[target] = 4294967295u32; } } state.table.free_list_head = 0u32; state.table.active_count = 0u32; }\n\"\"\"Returns the first target of the first record plus the active count of a borrowed Table, widened to u64, as a cheap checksum of the table.\"\"\"\nverb checksum(abs table: Table) -> u64 { return table.records[0u32].targets[0u32] as u64 + table.active_count as u64; }\n\"\"\"Copies the whole table of a State into a Snapshot: source mappings, every record target, used count, reserved slots and free-list link, the free list head and active count, and finally stores the table checksum in the snapshot.\"\"\"\nverb capture(ins snapshot: Snapshot, abs state: State) { for erg source: u32 in 0u32 .. 1024u32 { snapshot.table.source_to_record[source] = state.table.source_to_record[source]; } for erg record: u32 in 0u32 .. 8u32 { for erg target: u32 in 0u32 .. 8u32 { snapshot.table.records[record].targets[target] = state.table.records[record].targets[target]; } snapshot.table.records[record].used_count = state.table.records[record].used_count; snapshot.table.records[record].reserved[0u32] = state.table.records[record].reserved[0u32]; snapshot.table.records[record].reserved[1u32] = state.table.records[record].reserved[1u32]; snapshot.table.records[record].reserved[2u32] = state.table.records[record].reserved[2u32]; snapshot.table.records[record].free_list_link = state.table.records[record].free_list_link; } snapshot.table.free_list_head = state.table.free_list_head; snapshot.table.active_count = state.table.active_count; snapshot.checksum = checksum(table: abs state.table); }\n\"\"\"\ncontract:\npurpose:\n    Copy a Snapshot back into a State table when the snapshot checksum is the expected 1025.\ninputs:\n    state: exclusive ins loan of State whose table is overwritten; snapshot: borrowed Snapshot to restore from.\noutputs:\n    true when the table was restored, false when the checksum does not match.\nownership:\n    The state is an exclusive ins loan and is mutated; the snapshot is a borrowed abs view and is only read.\nerrors:\n    A checksum other than 1025 is reported as false and leaves the state untouched.\nside_effects:\n    Overwrites the state table on success.\n\"\"\"\nverb restore(ins state: State, abs snapshot: Snapshot) -> Bool { if snapshot.checksum != 1025u64 { return false; } for erg source: u32 in 0u32 .. 1024u32 { state.table.source_to_record[source] = snapshot.table.source_to_record[source]; } for erg record: u32 in 0u32 .. 8u32 { for erg target: u32 in 0u32 .. 8u32 { state.table.records[record].targets[target] = snapshot.table.records[record].targets[target]; } state.table.records[record].used_count = snapshot.table.records[record].used_count; state.table.records[record].reserved[0u32] = snapshot.table.records[record].reserved[0u32]; state.table.records[record].reserved[1u32] = snapshot.table.records[record].reserved[1u32]; state.table.records[record].reserved[2u32] = snapshot.table.records[record].reserved[2u32]; state.table.records[record].free_list_link = snapshot.table.records[record].free_list_link; } state.table.free_list_head = snapshot.table.free_list_head; state.table.active_count = snapshot.table.active_count; return true; }\n\"\"\"Entry point that fills one State of a Brain, captures it into a Snapshot, restores the snapshot into a freshly initialized State, and returns 42 when the restored link and source mapping match, 43 or 44 on a mismatch, or 45 when the restore is refused.\"\"\"\nverb main() -> Int { erg brain: Brain = Brain { prefix: Array[u8, 524288](), auxiliary: Array[State, 2](), }; erg index: u32 = 0u32; initialize(state: ins brain.auxiliary[index]); brain.auxiliary[index].table.source_to_record[0u32] = 0u16; brain.auxiliary[index].table.records[0u32].targets[0u32] = 1024u32; brain.auxiliary[index].table.active_count = 1u32; erg snapshot: Snapshot = Snapshot { table: Table { source_to_record: Array[u16, 1024](), records: Array[Record, 8](), free_list_head: 0u32, active_count: 0u32, }, checksum: 0u64, }; capture(snapshot: ins snapshot, state: abs brain.auxiliary[index]); erg recovered: State = State { table: Table { source_to_record: Array[u16, 1024](), records: Array[Record, 8](), free_list_head: 0u32, active_count: 0u32, }, }; initialize(state: ins recovered); if !restore(state: ins recovered, snapshot: abs snapshot) { return 45; } return if recovered.table.records[0u32].targets[0u32] == 1024u32 { if recovered.table.source_to_record[0u32] == 0u16 { 42 } else { 43 } } else { 44 }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-large-nested-snapshot-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create large nested snapshot fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"large-nested-snapshot-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write large nested snapshot manifest");
        fs::write(&input, source).expect("write large nested snapshot source");
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
        assert_eq!(result, 0, "backend {backend} should build large nested snapshot");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run large nested snapshot fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} large nested snapshot result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_large_aggregate_array_element_self_copy() {
    let source = "struct Cell { erg storage: Array[u8, 64], } struct Fabric { erg columns: Array[Cell, 1024], } struct Brain { erg fabrics: Array[Fabric, 2], }\n\"\"\"Entry point that copies a Fabric out of an array of a Brain, stores it back and returns 42.\"\"\"\nverb main() -> Int { erg brain: Brain = Brain { fabrics: Array[Fabric, 2](), }; erg local_fabric: Fabric = brain.fabrics[0u32]; brain.fabrics[0u32] = local_fabric; return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-{backend}-large-aggregate-array-self-copy-{}",
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create large aggregate self-copy fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"large-aggregate-array-self-copy-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write large aggregate self-copy manifest");
        fs::write(&input, source).expect("write large aggregate self-copy source");
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
        assert_eq!(result, 0, "backend {backend} should build large aggregate self-copy");
        let status = std::process::Command::new(&output)
            .status()
            .expect("run large aggregate self-copy fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} large aggregate self-copy");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_and_run_a_shared_nested_aggregate_array_element_store() {
    let source = "struct Point { x: u32, } struct Line { start: Point, }\n\"\"\"Entry point that stores a Line with a start Point of x 41 in an array and returns the x of that start plus 1.\"\"\"\nverb main() -> Int { erg values: Array[Line, 2] = Array[Line, 2](); values[1u32] = Line { start: Point { x: 41u32, }, }; return values[1u32].start.x as Int + 1; }\n";
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
    let source = "struct Box[T] { item: T, }\n\"\"\"Entry point that stores a generic Box of Int holding 41 in an array and returns its item plus 1.\"\"\"\nverb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: 41, }; return boxes[1u32].item + 1; }\n";
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
    let source = "enum Failure { Bad, } struct Counter { value: u32, }\n\"\"\"Entry point that builds an Ok result holding a Counter as an expression statement, discards it, and returns 42.\"\"\"\nverb main() -> Int { Result[Counter, Failure].Ok(Counter { value: 41u32, }); return 42; }\n";
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
    let source = "\"\"\"Entry point that builds an Option holding a borrowed u32 and a None of the same type as expression statements, discards them, and returns 42.\"\"\"\nverb main() -> Int { Option[abs u32].Some(41u32); Option[abs u32].None; return 42; }\n";
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
    let source = "struct Point { x: Int, }\n\"\"\"Adds 1 to the x field of element 1 of an exclusively borrowed array of Point structs and returns early.\"\"\"\nverb mutate(ins values: Array[Point, 2]) { values[1u32].x += 1; return; }\n\"\"\"Entry point that stores a Point with x 41 in an array, increments it through mutate and returns the x of that element.\"\"\"\nverb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); values[1u32] = Point { x: 41, }; mutate(values: ins values); return values[1u32].x; }\n";
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
    let source = "struct Box[T] { erg item: T, }\n\"\"\"Entry point that stores a Box of Int holding 41 in an array, adds 1 to its item with a compound assignment and returns the item.\"\"\"\nverb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: 41, }; boxes[1u32].item += 1; return boxes[1u32].item; }\n";
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
    let source = "struct Box[T] { item: T, } struct Pair[T] { item: T, }\n\"\"\"Entry point that stores a nested Pair of Box of Int holding 41 in an array and returns the innermost item plus 1.\"\"\"\nverb main() -> Int { erg pairs: Array[Pair[Box[Int]], 2] = Array[Pair[Box[Int]], 2](); pairs[1u32] = Pair[Box[Int]] { item: Box[Int] { item: 41, }, }; return pairs[1u32].item.item + 1; }\n";
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
    let source = "struct Box[T] { abs item: Option[abs T], }\n\"\"\"Entry point that stores a Box holding Option None in an array and returns 42 when the case sees None, otherwise 0.\"\"\"\nverb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: Option[abs Int].None, }; return case boxes[1u32].item { Option.Some(_) => 0, Option.None => 42, }; }\n";
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
    let source = "struct Box[T] { erg item: T, }\n\"\"\"\ncontract:\npurpose:\n    Take ownership of a Buffer and release it with drop.\ninputs:\n    value: owned Buffer that is dropped.\noutputs:\n    No value.\nownership:\n    The buffer is moved into the verb and dropped, so the caller cannot use it afterwards.\nside_effects:\n    Releases the buffer storage.\n\"\"\"\nverb consume(dat value: Buffer) { drop(value); return; }\n\"\"\"Entry point that builds a Box holding a one-byte Buffer, moves the buffer out of the box into consume and returns 42.\"\"\"\nverb main() -> Int { erg boxed: Box[Buffer] = Box[Buffer] { item: Buffer[1], }; consume(value: dat boxed.item); return 42; }\n";
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
    let source = "struct Box[T] { erg item: T, } struct Pair[T] { erg item: T, }\n\"\"\"\ncontract:\npurpose:\n    Take ownership of a Buffer and release it with drop.\ninputs:\n    value: owned Buffer that is dropped.\noutputs:\n    No value.\nownership:\n    The buffer is moved into the verb and dropped, so the caller cannot use it afterwards.\nside_effects:\n    Releases the buffer storage.\n\"\"\"\nverb consume(dat value: Buffer) { drop(value); return; }\n\"\"\"Entry point that builds a Pair of Box holding a one-byte Buffer, moves the buffer out of the nested item into consume and returns 42.\"\"\"\nverb main() -> Int { erg pair: Pair[Box[Buffer]] = Pair[Box[Buffer]] { item: Box[Buffer] { item: Buffer[1], }, }; consume(value: dat pair.item.item); return 42; }\n";
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
        "\"\"\"Stores 41 in element 2 of an exclusively borrowed four-element u32 array.\"\"\"\nverb mutate(ins values: Array[u32, 4]) { values[2u32] = 41u32; }\n\"\"\"Entry point that fills element 2 of a u32 array through mutate and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
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
        "\"\"\"Adds 41 to element 2 of an exclusively borrowed four-element u32 array with a compound assignment.\"\"\"\nverb mutate(ins values: Array[u32, 4]) { values[2u32] += 41u32; }\n\"\"\"Entry point that fills element 2 of a u32 array through mutate and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
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
        "\"\"\"Stores 41 in the element of an exclusively borrowed u32 array selected by the local index 2.\"\"\"\nverb mutate(ins values: Array[u32, 4]) { erg index: u32 = 2u32; values[index] = 41u32; }\n\"\"\"Entry point that fills element 2 of a u32 array through mutate and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); mutate(values: ins values); return values[2u32] as Int + 1; }\n",
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
        "struct Point { x: u32, }\n\"\"\"Stores 41 in the x field of element 1 of an exclusively borrowed array of Point structs.\"\"\"\nverb mutate(ins values: Array[Point, 2]) { values[1u32].x = 41u32; }\n\"\"\"Returns the x field of element 1 of a borrowed array of Point structs, as Int.\"\"\"\nverb read(abs values: Array[Point, 2]) -> Int { return values[1u32].x as Int; }\n\"\"\"Entry point that sets the x field of element 1 through mutate, reads it through read and returns it plus 1.\"\"\"\nverb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); mutate(values: ins values); return read(values: abs values) + 1; }\n",
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
