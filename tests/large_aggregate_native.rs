#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
use std::fs;

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
use actus::cli::run_with_args;

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn large_fixed_array_uses_runtime_storage_on_both_native_backends() {
    const LARGE_ARRAY_CAPACITY: usize = 64 * 1024;
    let source = format!(
        "struct Cell {{ erg value: u8, }} verb main() -> Int {{ erg cells: Array[Cell, {LARGE_ARRAY_CAPACITY}] = Array[Cell, {LARGE_ARRAY_CAPACITY}](); cells[{last_index}].value = 41u8; return cells[{last_index}].value as Int + 1; }}\n",
        last_index = LARGE_ARRAY_CAPACITY - 1,
    );
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-large-aggregate-{backend}-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.bin");
        fs::create_dir_all(&root).expect("create large aggregate fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"large-aggregate-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\n"
            ),
        )
        .expect("write large aggregate manifest");
        fs::write(&input, &source).expect("write large aggregate source");
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
        assert_eq!(result, 0, "{backend} large aggregate build should pass");
        let status =
            std::process::Command::new(&output).status().expect("run large aggregate fixture");
        assert_eq!(status.code(), Some(42), "{backend} large aggregate runtime should pass");
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(unix, feature = "cranelift", feature = "llvm"))]
#[test]
fn freestanding_target_rejects_implicit_large_aggregate_allocation() {
    let source = "struct Cell { erg value: u8, } verb main() -> Int { erg cells: Array[Cell, 131072] = Array[Cell, 131072](); return cells[0].value as Int; }\n";
    for backend in ["cranelift", "llvm"] {
        let root = std::env::temp_dir()
            .join(format!("actus-large-aggregate-freestanding-{backend}-{}", std::process::id()));
        let input = root.join("main.act");
        let output = root.join("main.o");
        fs::create_dir_all(&root).expect("create freestanding fixture directory");
        fs::write(
            root.join("Actus.toml"),
            format!(
                "[package]\nname = \"large-aggregate-freestanding-{backend}\"\nversion = \"1.0.0\"\n\n[build]\nbackend = \"{backend}\"\ntarget = \"thumbv7em-none-eabihf\"\n"
            ),
        )
        .expect("write freestanding aggregate manifest");
        fs::write(&input, source).expect("write freestanding aggregate source");
        let result = run_with_args(
            vec![
                "build".to_owned(),
                input.display().to_string(),
                "--strict".to_owned(),
                "--emit".to_owned(),
                "obj".to_owned(),
                "-o".to_owned(),
                output.display().to_string(),
            ]
            .into_iter(),
        );
        assert_ne!(result, 0, "{backend} must reject implicit large aggregate allocation");
        let _ = fs::remove_dir_all(root);
    }
}
