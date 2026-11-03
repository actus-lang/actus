use std::fs;
use std::path::PathBuf;
use std::process::Command;

use object::{Object, ObjectSymbol};

fn compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_actus"))
}

#[cfg(all(feature = "llvm", feature = "cranelift"))]
const NATIVE_BACKENDS: &[&str] = &["llvm", "cranelift"];
#[cfg(all(feature = "llvm", not(feature = "cranelift")))]
const NATIVE_BACKENDS: &[&str] = &["llvm"];
#[cfg(all(feature = "cranelift", not(feature = "llvm")))]
const NATIVE_BACKENDS: &[&str] = &["cranelift"];
#[cfg(not(any(feature = "llvm", feature = "cranelift")))]
const NATIVE_BACKENDS: &[&str] = &[];

#[test]
fn zero_on_drop_emits_secure_zero_on_both_native_backends() {
    let root = std::env::temp_dir().join(format!("actus-zero-on-drop-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create fixture source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"zero_on_drop_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nverify_no_float_ir = true\n",
    )
    .expect("write fixture manifest");
    fs::write(
        source_root.join("main.act"),
        r#"
meta repr(C)
meta zero_on_drop
struct Secret {
    key_a: u64,
    key_b: u64,
}

verb main() -> Int {
    erg secret: Secret = Secret { key_a: 1u64, key_b: 2u64, };
    return secret.key_a as Int;
}
"#,
    )
    .expect("write fixture source");

    for backend in NATIVE_BACKENDS {
        let output = root.join(format!("zero-on-drop-{backend}.o"));
        let build = Command::new(compiler())
            .current_dir(&root)
            .args(["build", "--strict", "--backend", backend, "--emit", "obj", "-o"])
            .arg(&output)
            .output()
            .expect("start Actus object build");
        assert!(
            build.status.success(),
            "{backend} backend failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let bytes = fs::read(&output).expect("read generated object");
        let object = object::File::parse(bytes.as_slice()).expect("parse generated object");
        let symbols = object
            .symbols()
            .filter(|symbol| symbol.is_undefined())
            .filter_map(|symbol| symbol.name().ok())
            .collect::<Vec<_>>();
        assert!(
            symbols.contains(&"actus_secure_zero"),
            "secure-zero runtime bridge missing for {backend}: {symbols:?}"
        );
    }
    fs::remove_dir_all(root).expect("remove zero_on_drop fixture");
}
