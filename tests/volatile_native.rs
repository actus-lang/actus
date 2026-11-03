use std::fs;
use std::path::PathBuf;
use std::process::Command;

use object::{Object, ObjectSymbol};

fn compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_actus"))
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

    let output = root.join("volatile-read.o");
    let build = Command::new(compiler())
        .current_dir(&root)
        .args(["build", "--strict", "--emit", "obj", "-o"])
        .arg(&output)
        .output()
        .expect("start Actus object build");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));

    let bytes = fs::read(&output).expect("read generated object");
    let object = object::File::parse(bytes.as_slice()).expect("parse generated object");
    let undefined = object
        .symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name().ok())
        .collect::<Vec<_>>();
    assert!(
        undefined.iter().any(|symbol| symbol.contains("volatile_5fread_5fu32")),
        "typed volatile read specialization missing: {undefined:?}"
    );
    assert!(undefined.contains(&"provider"), "trusted provider symbol missing: {undefined:?}");
    fs::remove_dir_all(root).expect("remove volatile read fixture");
}
