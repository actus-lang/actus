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

fn undefined_symbols(output: &PathBuf) -> Vec<String> {
    let bytes = fs::read(output).expect("read generated object");
    let object = object::File::parse(bytes.as_slice()).expect("parse generated object");
    object
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name().ok().map(str::to_owned))
        .collect()
}

#[test]
fn typed_volatile_read_fixture_emits_checked_native_call() {
    let root = std::env::temp_dir().join(format!("actus-volatile-read-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create fixture source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"volatile_read_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("write fixture manifest");
    fs::write(
        source_root.join("main.act"),
        r#"
import { open std::volatile };

meta repr(C)
meta volatile
struct Registers {
    status: u32,
    data: u32,
}

unsafe extern "C" verb provider() -> VolatileAddress[Registers];

meta effect("volatile")
verb main() -> Int {
    erg address = provider();
    erg result = volatile_read_u32[Registers, 4](address: abs address);
    return case dat result {
        Result.Ok(value) => value as Int,
        Result.Err(_) => -1,
    };
}
"#,
    )
    .expect("write fixture source");

    for backend in NATIVE_BACKENDS {
        let output = root.join(format!("volatile-read-{backend}.o"));
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

        let undefined = undefined_symbols(&output);
        assert!(
            undefined.iter().any(|symbol| symbol.contains("volatile_5fread_5fu32")),
            "typed volatile read specialization missing for {backend}: {undefined:?}"
        );
        assert!(
            undefined.iter().any(|symbol| symbol == "provider"),
            "trusted provider symbol missing for {backend}: {undefined:?}"
        );
    }
    fs::remove_dir_all(root).expect("remove volatile read fixture");
}

#[test]
fn typed_volatile_write_fixture_requires_capability_and_emits_checked_native_call() {
    let root = std::env::temp_dir().join(format!("actus-volatile-write-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create fixture source directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"volatile_write_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("write fixture manifest");
    fs::write(
        source_root.join("main.act"),
        r#"
import { open std::volatile };

meta repr(C)
meta volatile
struct Registers {
    status: u32,
    data: u32,
}

unsafe extern "C" verb provider() -> VolatileWriteAddress[Registers];

meta effect("volatile")
meta capability("volatile.write")
verb main() -> Int {
    erg address = provider();
    erg write_value: u32 = 7u32;
    erg result = volatile_write_u32[Registers, 4](address: ins address, value: erg write_value);
    return case dat result {
        Result.Ok(_) => 0,
        Result.Err(_) => 1,
    };
}
"#,
    )
    .expect("write fixture source");

    for backend in NATIVE_BACKENDS {
        let output = root.join(format!("volatile-write-{backend}.o"));
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

        let undefined = undefined_symbols(&output);
        assert!(
            undefined.iter().any(|symbol| symbol.contains("volatile_5fwrite_5fu32")),
            "typed volatile write specialization missing for {backend}: {undefined:?}"
        );
        assert!(
            undefined.iter().any(|symbol| symbol == "provider"),
            "trusted provider symbol missing for {backend}: {undefined:?}"
        );
    }
    fs::remove_dir_all(root).expect("remove volatile write fixture");
}
