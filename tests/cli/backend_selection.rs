use super::*;

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

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_preserve_local_assignment_in_an_if_without_else() {
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir().join(format!(
            "actus-cli-if-assignment-{}-{}",
            backend,
            std::process::id()
        ));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create if-assignment fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"if-assignment-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write if-assignment fixture manifest");
        fs::write(
            &input,
            "verb main() -> Int { erg value: u32 = 0u32; if true { value = 41u32; } return value as Int + 1; }\n",
        )
        .expect("write if-assignment fixture source");

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
        assert_eq!(result, 0, "backend {backend} should build the fixture");
        let status =
            std::process::Command::new(&output).status().expect("run if-assignment fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} should preserve the assignment");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_preserve_local_assignment_in_an_else_if() {
    let source = "verb main() -> Int { erg value: u32 = 0u32; if false { value = 7u32; } else if true { value = 41u32; } return value as Int + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-else-if-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write else-if fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build else-if");
        let status = std::process::Command::new(&output).status().expect("run else-if fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} else-if result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_preserve_local_assignment_in_both_if_branches() {
    let source = "verb main() -> Int { erg value: u32 = 0u32; if true { value = 41u32; } else { value = 7u32; } return value as Int + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-if-else-assignment-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write if/else assignment fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build if/else assignment");
        let status =
            std::process::Command::new(&output).status().expect("run if/else assignment fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} should preserve the merge value");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_standalone_case_statement_through_shared_lowering() {
    let source = "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb main() -> Int { erg result = produce(); case dat result { Result.Ok(value) => value, Result.Err(_) => 0, }; return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-standalone-case-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write standalone case fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build standalone case");
        let status =
            std::process::Command::new(&output).status().expect("run standalone case fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} standalone case result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_guarded_case_through_shared_lowering() {
    let source = "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb select_true() -> Int { erg result = produce(); return case result { Result.Ok(value) if value == 41 => 42, Result.Ok(_) => 1, Result.Err(_) => 0, }; } verb select_false() -> Int { erg result = produce(); return case result { Result.Ok(value) if value == 40 => 1, Result.Ok(_) => 42, Result.Err(_) => 0, }; } verb main() -> Int { return select_true() + select_false() - 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-guarded-case-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write guarded case fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build guarded case");
        let status =
            std::process::Command::new(&output).status().expect("run guarded case fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} should evaluate the guard");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_scalar_try_owner_through_shared_lowering() {
    let source = "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb worker() -> Result[Int, Failure] { erg value = produce()?; return Result[Int, Failure].Ok(value + 1); } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(value) => value, Result.Err(_) => 0, }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-try-owner-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write try-owner fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build scalar try owner");
        let status = std::process::Command::new(&output).status().expect("run try-owner fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} scalar try result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_scalar_try_return_through_shared_lowering() {
    let source = "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb worker() -> Result[Int, Failure] { return produce()?; } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(value) => value + 1, Result.Err(_) => 0, }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-try-return-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write try-return fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build scalar try return");
        let status = std::process::Command::new(&output).status().expect("run try-return fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} scalar try return result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_scalar_try_expression_through_shared_lowering() {
    let source = "verb produce() -> Result[Int, Int] { return Result[Int, Int].Err(9); } verb worker() -> Result[Int, Int] { produce()?; return Result[Int, Int].Ok(42); } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(_) => 0, Result.Err(error) => error, }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-try-expression-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write try-expression fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build scalar try expression");
        let status =
            std::process::Command::new(&output).status().expect("run try-expression fixture");
        assert_eq!(status.code(), Some(9), "backend {backend} should propagate Err");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_scalar_literal_case_through_shared_lowering() {
    let source =
        "verb main() -> Int { erg value: u32 = 41u32; return case value { 41 => 42, _ => 0, }; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-literal-case-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write literal-case fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build literal case");
        let status =
            std::process::Command::new(&output).status().expect("run literal-case fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} literal case result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_call_with_indexed_scalar_argument_through_shared_lowering() {
    let source = "verb observe(abs value: u32) { return; } verb main() -> Int { erg values: Array[u32, 2] = Array[u32, 2](); values[1u32] = 41u32; erg index: u32 = 1u32; observe(value: abs values[index]); return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-indexed-call-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write indexed-call fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build indexed call");
        let status =
            std::process::Command::new(&output).status().expect("run indexed-call fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} indexed-call result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_standalone_result_constructor_through_shared_lowering() {
    let source =
        "enum Failure { Bad, } verb main() -> Int { Result[Int, Failure].Ok(41); return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-standalone-result-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write standalone Result fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build standalone Result");
        let status =
            std::process::Command::new(&output).status().expect("run standalone Result fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} standalone Result result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_string_literal_through_shared_lowering() {
    let source = "verb main() -> Int { erg text = \"Actus shared string\"; return 42; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-string-literal-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write string literal fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build string literal");
        let status =
            std::process::Command::new(&output).status().expect("run string literal fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} string literal result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_nested_option_case_arithmetic_through_shared_lowering() {
    let source = "struct Node { value: Int, abs next: Option[abs Node], } verb sum(abs node: Node) -> Int { abs next = ref node.next; return node.value + case next { Option.Some(_) => 1, Option.None => 0, }; } verb main() -> Int { erg node = Node { value: 41, next: Option[abs Node].None, }; return sum(node: abs node); }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-nested-option-case-{backend}-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create nested Option case fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"nested-option-case-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write nested Option case manifest");
        fs::write(&input, source).expect("write nested Option case source");
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
        assert_eq!(result, 0, "backend {backend} should build nested Option case");
        let status =
            std::process::Command::new(&output).status().expect("run nested Option case fixture");
        assert_eq!(status.code(), Some(41), "backend {backend} nested Option case result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_nested_array_and_range_loops_through_shared_lowering() {
    let source = "verb main() -> Int { erg values: Array[u32, 2] = Array[u32, 2](); values[0u32] = 20u32; values[1u32] = 21u32; erg total: u32 = 0u32; for erg outer: u32 in values { for erg inner: u32 in 0u32 .. 1u32 { total += values[outer] + inner; } } return total as Int; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-nested-loops-{backend}-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create nested loop fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"nested-loops-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write nested loop fixture manifest");
        fs::write(&input, source).expect("write nested loop fixture source");

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
        assert_eq!(result, 0, "backend {backend} should build nested loops");
        let status = std::process::Command::new(&output).status().expect("run nested loop fixture");
        assert_eq!(status.code(), Some(41), "backend {backend} nested loop result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_nested_range_and_array_loops_through_shared_lowering() {
    let source = "verb main() -> Int { erg values: Array[u32, 2] = Array[u32, 2](); values[0u32] = 20u32; values[1u32] = 21u32; erg total: u32 = 0u32; for erg outer: u32 in 0u32 .. 2u32 { for erg inner: u32 in values { total += values[inner] + outer; } } return total as Int; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-nested-range-array-{backend}-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create nested range-array fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"nested-range-array-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write nested range-array fixture manifest");
        fs::write(&input, source).expect("write nested range-array fixture source");

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
        assert_eq!(result, 0, "backend {backend} should build nested range-array loops");
        let status =
            std::process::Command::new(&output).status().expect("run nested range-array fixture");
        assert_eq!(status.code(), Some(84), "backend {backend} nested range-array result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_build_range_loop_exits_through_shared_lowering() {
    let source = "verb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 0u32 .. 4u32 { if index == 1u32 { continue; } if index == 3u32 { break; } total += index; } return total as Int + 40; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-range-exits-{backend}-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create range exit fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"range-exits-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write range exit fixture manifest");
        fs::write(&input, source).expect("write range exit fixture source");

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
        assert_eq!(result, 0, "backend {backend} should build range exits");
        let status = std::process::Command::new(&output).status().expect("run range exit fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} range exit result");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn both_native_backends_share_scalar_ins_caller_storage_and_writeback() {
    let source = "verb mutate(ins value: Int) { value = 41; } verb main() -> Int { erg value: Int = 0; mutate(value: ins value); return value + 1; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-cli-scalar-ins-{backend}-{}", std::process::id()));
        let input = root.with_extension("act");
        let output = root.with_extension("bin");
        fs::write(&input, source).expect("write scalar-ins fixture source");
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
        assert_eq!(result, 0, "backend {backend} should build scalar-ins fixture");
        let status = std::process::Command::new(&output).status().expect("run scalar-ins fixture");
        assert_eq!(status.code(), Some(42), "backend {backend} scalar-ins result");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
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
