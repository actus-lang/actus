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

#[cfg(all(unix, feature = "cranelift"))]
#[test]
fn cranelift_backend_lowers_case_branch_with_trailing_if_value() {
    let root = std::env::temp_dir()
        .join(format!("actus-cli-cranelift-case-if-tail-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "enum Failure { Bad, } verb check(abs value: Int) -> Bool { erg result: Result[Int, Failure] = Result[Int, Failure].Ok(1); return case dat result { Result.Err(_) => false, Result.Ok(_) => { if value == 0 { false; } else { erg selected: Bool = true; selected; } }, }; } verb main() -> Int { erg value = 1; if check(value: abs value) { return 0; } return 1; }\n",
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
        "enum Failure { Bad, } verb check(abs value: Int) -> Bool { erg result: Result[Int, Failure] = Result[Int, Failure].Ok(1); return case dat result { Result.Err(_) => false, Result.Ok(_) => { if value == 0 { false; } else { erg selected: Bool = true; selected; } }, }; } verb main() -> Int { erg value = 1; if check(value: abs value) { return 0; } return 1; }\n",
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
        "enum Failure { Bad, } verb consume(abs value: u32) -> Int { return value as Int; } verb check() -> Int { erg target: u32 = 0u32; erg result: Result[u32, Failure] = Result[u32, Failure].Ok(1031u32); erg status: Int = case dat result { Result.Err(_) => 1, Result.Ok(index) => { target = index; 0; }, }; return consume(value: abs target) + status; } verb main() -> Int { if check() == 1031 { return 0; } return 1; }\n",
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
        "enum Failure { Bad, } verb consume(abs value: u32) -> Int { return value as Int; } verb check() -> Int { erg target: u32 = 0u32; erg result: Result[u32, Failure] = Result[u32, Failure].Ok(1031u32); erg status: Int = case dat result { Result.Err(_) => 1, Result.Ok(index) => { target = index; 0; }, }; return consume(value: abs target) + status; } verb main() -> Int { if check() == 1031 { return 0; } return 1; }\n",
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
fn cranelift_backend_coerces_case_branch_values_to_the_merge_type() {
    let root =
        std::env::temp_dir().join(format!("actus-cli-cranelift-case-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(
        &input,
        "verb main() -> Int { erg left: u8 = 13; erg right: u8 = 5; erg value = case (left < right) || (left <= right) { true => 1, _ => 0, }; return value; }\n",
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
    let source = "verb main() -> Int { erg left: u8 = 13u8; erg right: u8 = 5u8; erg value = case (left < right) || (left <= right) { true => 1, _ => 0, }; return value; }\n";
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
    let source = "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(7); } verb main() -> Int { erg result = produce(); return case dat result { Result.Err(_) => { return 1; }, Result.Ok(_) => {}, }; return if 7 == 7 { 0 } else { 1 }; }\n";
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

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_share_structured_ins_caller_owned_storage() {
    let source = "struct Counter { value: Int, } verb increment(ins counter: Counter) { counter.value += 1; return; } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; increment(counter: ins counter); return counter.value; }\n";
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
