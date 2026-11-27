#[cfg(unix)]
use super::*;

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_builds_and_runs_a_shared_scalar_struct_parameter_read() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-struct-param-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, }\n\"\"\"Returns the value field of a borrowed Counter.\"\"\"\nverb read(abs counter: Counter) -> Int { return counter.value; }\n\"\"\"Entry point that builds a Counter holding 41, reads it through a borrow and returns the value plus 1.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; return read(counter: abs counter) + 1; }\n",
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
        "struct Counter { value: Int, }\n\"\"\"Returns the value field of a borrowed Counter.\"\"\"\nverb read(abs counter: Counter) -> Int { return counter.value; }\n\"\"\"Entry point that builds a Counter holding 41, reads it through a borrow and returns the value plus 1.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; return read(counter: abs counter) + 1; }\n",
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
        "\"\"\"Entry point that stores 41 in element 2 of a u32 array with a plain assignment and returns that element as Int plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] = 41u32; return values[2u32] as Int + 1; }\n",
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
        "\"\"\"Entry point that stores 41 in element 2 of a u32 array with a plain assignment and returns that element as Int plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] = 41u32; return values[2u32] as Int + 1; }\n",
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
        "\"\"\"Entry point that adds 41 to element 2 of a u32 array with a compound assignment and returns that element as Int plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] += 41u32; return values[2u32] as Int + 1; }\n",
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

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_lowers_case_branch_with_trailing_if_value() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-case-if-tail-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "enum Failure { Bad, }\n\"\"\"\ncontract:\npurpose:\n    Evaluate a case over an Ok result whose block arm picks true or false from the borrowed value, to check block tails in case arms.\ninputs:\n    value: borrowed Int compared with zero.\noutputs:\n    false when the value is 0, true otherwise.\nownership:\n    The value is a borrowed abs view and is only read.\n\"\"\"\nverb check(abs value: Int) -> Bool { erg result: Result[Int, Failure] = Result[Int, Failure].Ok(1); return case dat result { Result.Err(_) => false, Result.Ok(_) => { if value == 0 { false; } else { erg selected: Bool = true; selected; } }, }; }\n\"\"\"Entry point that returns 0 when check accepts the value 1, and 1 otherwise.\"\"\"\nverb main() -> Int { erg value = 1; if check(value: abs value) { return 0; } return 1; }\n",
    )
    .expect("write Cranelift case-if tail fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "cranelift",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is valid UTF-8"),
            input.to_str().expect("input path is valid UTF-8"),
        ])
        .status()
        .expect("build Cranelift case-if tail fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run Cranelift case-if tail fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_lowers_case_branch_with_trailing_if_value() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-case-if-tail-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "enum Failure { Bad, }\n\"\"\"\ncontract:\npurpose:\n    Evaluate a case over an Ok result whose block arm picks true or false from the borrowed value, to check block tails in case arms.\ninputs:\n    value: borrowed Int compared with zero.\noutputs:\n    false when the value is 0, true otherwise.\nownership:\n    The value is a borrowed abs view and is only read.\n\"\"\"\nverb check(abs value: Int) -> Bool { erg result: Result[Int, Failure] = Result[Int, Failure].Ok(1); return case dat result { Result.Err(_) => false, Result.Ok(_) => { if value == 0 { false; } else { erg selected: Bool = true; selected; } }, }; }\n\"\"\"Entry point that returns 0 when check accepts the value 1, and 1 otherwise.\"\"\"\nverb main() -> Int { erg value = 1; if check(value: abs value) { return 0; } return 1; }\n",
    )
    .expect("write LLVM case-if tail fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "llvm",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is valid UTF-8"),
            input.to_str().expect("input path is valid UTF-8"),
        ])
        .status()
        .expect("build LLVM case-if tail fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run LLVM case-if tail fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_merges_case_branch_assignment_before_call() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-case-assignment-call-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "enum Failure { Bad, }\n\"\"\"Returns a borrowed u32 converted to Int.\"\"\"\nverb consume(abs value: u32) -> Int { return value as Int; }\n\"\"\"\ncontract:\npurpose:\n    Assign the Ok payload of a Result to an outer variable inside a case block and return that value plus the case status.\noutputs:\n    1031 when the Result is Ok, since the status is then 0.\nownership:\n    The target is a local u32 written from the case block and then borrowed by consume.\n\"\"\"\nverb check() -> Int { erg target: u32 = 0u32; erg result: Result[u32, Failure] = Result[u32, Failure].Ok(1031u32); erg status: Int = case dat result { Result.Err(_) => 1, Result.Ok(index) => { target = index; 0; }, }; return consume(value: abs target) + status; }\n\"\"\"Entry point that returns 0 when check yields 1031, and 1 otherwise.\"\"\"\nverb main() -> Int { if check() == 1031 { return 0; } return 1; }\n",
    )
    .expect("write Cranelift case-assignment call fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "cranelift",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is valid UTF-8"),
            input.to_str().expect("input path is valid UTF-8"),
        ])
        .status()
        .expect("build Cranelift case-assignment call fixture");
    assert!(status.success());
    let status =
        Command::new(&output).status().expect("run Cranelift case-assignment call fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_merges_case_branch_assignment_before_call() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-llvm-case-assignment-call-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "enum Failure { Bad, }\n\"\"\"Returns a borrowed u32 converted to Int.\"\"\"\nverb consume(abs value: u32) -> Int { return value as Int; }\n\"\"\"\ncontract:\npurpose:\n    Assign the Ok payload of a Result to an outer variable inside a case block and return that value plus the case status.\noutputs:\n    1031 when the Result is Ok, since the status is then 0.\nownership:\n    The target is a local u32 written from the case block and then borrowed by consume.\n\"\"\"\nverb check() -> Int { erg target: u32 = 0u32; erg result: Result[u32, Failure] = Result[u32, Failure].Ok(1031u32); erg status: Int = case dat result { Result.Err(_) => 1, Result.Ok(index) => { target = index; 0; }, }; return consume(value: abs target) + status; }\n\"\"\"Entry point that returns 0 when check yields 1031, and 1 otherwise.\"\"\"\nverb main() -> Int { if check() == 1031 { return 0; } return 1; }\n",
    )
    .expect("write LLVM case-assignment call fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "llvm",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is valid UTF-8"),
            input.to_str().expect("input path is valid UTF-8"),
        ])
        .status()
        .expect("build LLVM case-assignment call fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run LLVM case-assignment call fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_copies_aggregate_case_payload_into_existing_owner() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-aggregate-case-assignment-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Location { erg global_index: u32, } enum Failure { Invalid, }\n\"\"\"\ncontract:\npurpose:\n    Overwrite an exclusively borrowed Location with the Location carried by an Ok result, or reset its index to zero on error.\ninputs:\n    location: exclusive ins loan of Location replaced or reset in place.\noutputs:\n    No value.\nownership:\n    The location is an exclusive ins loan and is mutated.\nside_effects:\n    Replaces the whole Location, or sets global_index to 0.\n\"\"\"\nverb assign(ins location: Location) { erg result = Result[Location, Failure].Ok(Location { global_index: 41u32, }); case dat result { Result.Err(_) => { location.global_index = 0u32; }, Result.Ok(value) => { location = value; }, }; }\n\"\"\"Entry point that starts with a zeroed Location, lets assign overwrite it and returns the resulting global_index as Int.\"\"\"\nverb main() -> Int { erg location: Location = Location { global_index: 0u32, }; assign(location: ins location); return location.global_index as Int; }\n",
    )
    .expect("write Cranelift aggregate case assignment fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "cranelift",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is valid UTF-8"),
            input.to_str().expect("input path is valid UTF-8"),
        ])
        .status()
        .expect("build Cranelift aggregate case assignment fixture");
    assert!(status.success());
    let status =
        Command::new(&output).status().expect("run Cranelift aggregate case assignment fixture");
    assert_eq!(status.code(), Some(41));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_copies_aggregate_case_payload_into_existing_owner() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-llvm-aggregate-case-assignment-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Location { erg global_index: u32, } enum Failure { Invalid, }\n\"\"\"\ncontract:\npurpose:\n    Overwrite an exclusively borrowed Location with the Location carried by an Ok result, or reset its index to zero on error.\ninputs:\n    location: exclusive ins loan of Location replaced or reset in place.\noutputs:\n    No value.\nownership:\n    The location is an exclusive ins loan and is mutated.\nside_effects:\n    Replaces the whole Location, or sets global_index to 0.\n\"\"\"\nverb assign(ins location: Location) { erg result = Result[Location, Failure].Ok(Location { global_index: 41u32, }); case dat result { Result.Err(_) => { location.global_index = 0u32; }, Result.Ok(value) => { location = value; }, }; }\n\"\"\"Entry point that starts with a zeroed Location, lets assign overwrite it and returns the resulting global_index as Int.\"\"\"\nverb main() -> Int { erg location: Location = Location { global_index: 0u32, }; assign(location: ins location); return location.global_index as Int; }\n",
    )
    .expect("write LLVM aggregate case assignment fixture");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            "--strict",
            "--backend",
            "llvm",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path is valid UTF-8"),
            input.to_str().expect("input path is valid UTF-8"),
        ])
        .status()
        .expect("build LLVM aggregate case assignment fixture");
    assert!(status.success());
    let status =
        Command::new(&output).status().expect("run LLVM aggregate case assignment fixture");
    assert_eq!(status.code(), Some(41));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_coerces_case_branch_values_to_the_merge_type() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-case-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "\"\"\"Entry point that compares two u8 values with untyped literals through a short-circuit or and returns 1 when either comparison holds, otherwise 0.\"\"\"\nverb main() -> Int { erg left: u8 = 13; erg right: u8 = 5; erg value = case (left < right) || (left <= right) { true => 1, _ => 0, }; return value; }\n",
    )
    .expect("write Cranelift case merge fixture");
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
        .expect("build Cranelift case merge fixture");
    assert!(status.success());
    let status = Command::new(&output).status().expect("run Cranelift case merge fixture");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_share_logical_case_predicates() {
    let source = "\"\"\"Entry point that compares two u8 values with typed literals through a short-circuit or and returns 1 when either comparison holds, otherwise 0.\"\"\"\nverb main() -> Int { erg left: u8 = 13u8; erg right: u8 = 5u8; erg value = case (left < right) || (left <= right) { true => 1, _ => 0, }; return value; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-logical-case-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create logical case directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"logical-case-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write logical case manifest");
        fs::write(&input, source).expect("write logical case source");
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
        assert_eq!(result, 0, "backend {backend} should build logical case fixture");
        let status =
            std::process::Command::new(&output).status().expect("run logical case fixture");
        assert_eq!(status.code(), Some(0), "backend {backend} logical case result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_share_scalar_if_and_empty_case_continuation() {
    let source = "enum Failure { Bad, }\n\"\"\"\ncontract:\npurpose:\n    Return an Ok result holding 7.\noutputs:\n    Result.Ok with 7.\n\"\"\"\nverb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(7); }\n\"\"\"Entry point that returns 1 from inside a case arm when produce fails and falls through the Ok arm to return 0 when the check 7 equals 7 holds.\"\"\"\nverb main() -> Int { erg result = produce(); return case dat result { Result.Err(_) => { return 1; }, Result.Ok(_) => {}, }; return if 7 == 7 { 0 } else { 1 }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-scalar-if-case-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create scalar if case directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"scalar-if-case-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write scalar if case manifest");
        fs::write(&input, source).expect("write scalar if case source");
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
        assert_eq!(result, 0, "backend {backend} should build scalar if case fixture");
        let status =
            std::process::Command::new(&output).status().expect("run scalar if case fixture");
        assert_eq!(status.code(), Some(0), "backend {backend} scalar if case result");
        let _ = fs::remove_dir_all(root);
    }
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
        "\"\"\"Entry point that adds 41 to element 2 of a u32 array with a compound assignment and returns that element as Int plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] += 41u32; return values[2u32] as Int + 1; }\n",
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
        "\"\"\"Adds 41 to the element of a fresh u32 array selected by a borrowed index and returns that element plus 1.\"\"\"\nverb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[index] += 41u32; return values[index] as Int + 1; }\n\"\"\"Entry point that returns the result of read for the in-range index 2.\"\"\"\nverb main() -> Int { return read(index: abs 2u32); }\n",
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
        "\"\"\"Adds 41 to the element of a fresh u32 array selected by a borrowed index and returns that element plus 1.\"\"\"\nverb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[index] += 41u32; return values[index] as Int + 1; }\n\"\"\"Entry point that returns the result of read for the in-range index 2.\"\"\"\nverb main() -> Int { return read(index: abs 2u32); }\n",
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
fn cranelift_backend_reports_a_shared_dynamic_fixed_array_index_out_of_bounds() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-array-oob-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "\"\"\"Returns the element of a fresh u32 array selected by a borrowed index, which the test calls with an out-of-range index.\"\"\"\nverb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[index] as Int; }\n\"\"\"Entry point that calls read with the index 4, one past the end of the four-element array.\"\"\"\nverb main() -> Int { return read(index: abs 4u32); }\n",
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
    let execution = Command::new(&output).output().expect("run Cranelift array bounds fixture");
    assert_eq!(execution.status.code(), Some(101), "array bounds failure must be reported");
    let stderr = String::from_utf8_lossy(&execution.stderr);
    assert!(stderr.starts_with("error: index out of bounds at "), "reported {stderr:?}");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_reports_a_shared_dynamic_fixed_array_index_out_of_bounds() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-llvm-array-oob-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "\"\"\"Returns the element of a fresh u32 array selected by a borrowed index, which the test calls with an out-of-range index.\"\"\"\nverb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[index] as Int; }\n\"\"\"Entry point that calls read with the index 4, one past the end of the four-element array.\"\"\"\nverb main() -> Int { return read(index: abs 4u32); }\n",
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
    let execution = Command::new(&output).output().expect("run LLVM array bounds fixture");
    assert_eq!(execution.status.code(), Some(101), "array bounds failure must be reported");
    let stderr = String::from_utf8_lossy(&execution.stderr);
    assert!(stderr.starts_with("error: index out of bounds at "), "reported {stderr:?}");
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
        "struct Counter { value: Int, }\n\"\"\"Adds 1 to the value of an exclusively borrowed Counter and returns early.\"\"\"\nverb increment(ins counter: Counter) { counter.value += 1; return; }\n\"\"\"Entry point that increments a Counter holding 41 through increment and returns its value.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; increment(counter: ins counter); return counter.value; }\n",
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
        "struct Counter { value: Int, }\n\"\"\"Adds 1 to the value of an exclusively borrowed Counter and returns early.\"\"\"\nverb increment(ins counter: Counter) { counter.value += 1; return; }\n\"\"\"Entry point that increments a Counter holding 41 through increment and returns its value.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; increment(counter: ins counter); return counter.value; }\n",
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

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_share_structured_ins_caller_owned_storage() {
    let source = "struct Counter { value: Int, }\n\"\"\"Adds 1 to the value of an exclusively borrowed Counter and returns early.\"\"\"\nverb increment(ins counter: Counter) { counter.value += 1; return; }\n\"\"\"Entry point that increments a Counter holding 41 through increment and returns its value.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; increment(counter: ins counter); return counter.value; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-{backend}-structured-ins-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create structured ins directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"structured-ins-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write structured ins manifest");
        fs::write(&input, source).expect("write structured ins source");
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
        assert_eq!(result, 0, "backend {backend} should build structured ins fixture");
        let status =
            std::process::Command::new(&output).status().expect("run structured ins fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} structured ins result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "llvm"))]
#[test]
fn llvm_backend_builds_and_runs_scalar_aggregate_field_access() {
    let root = std::env::temp_dir().join(format!("actus-cli-llvm-field-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "struct Counter { value: Int, }\n\"\"\"Entry point that returns the value of a Counter holding 41 plus 1.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; return counter.value + 1; }\n",
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
        "struct Counter { value: Int, }\n\"\"\"Entry point that assigns 42 to the value of a Counter holding 41 and returns it.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value = 42; return counter.value; }\n",
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
        "struct Counter { value: Int, }\n\"\"\"Entry point that adds 1 to the value of a Counter holding 41 with a compound assignment and returns it.\"\"\"\nverb main() -> Int { erg counter: Counter = Counter { value: 41, }; counter.value += 1; return counter.value; }\n",
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
    fs::write(&input, "\"\"\"Entry point that returns the bitwise and of 12 and 10 combined with 1 by bitwise or.\"\"\"\nverb main() -> Int { return (12 & 10) | 1; }\n")
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
    fs::write(&input, "\"\"\"Entry point that returns 20 divided by 5 plus the remainder of 17 divided by 6.\"\"\"\nverb main() -> Int { return 20 / 5 + 17 % 6; }\n")
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
    fs::write(&input, "\"\"\"Entry point that returns 1 shifted left by 4 plus 32 shifted right by 3.\"\"\"\nverb main() -> Int { return (1 << 4) + (32 >> 3); }\n")
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
