use std::fs;
use std::path::Path;
use std::process::Command;

fn write_fixture(root: &Path) {
    fs::create_dir_all(root.join("src/alpha")).unwrap();
    fs::create_dir_all(root.join("src/zeta")).unwrap();
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"reproducibility\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/main.act"),
        "import zeta; import alpha; import alpha; verb main() -> Int { return aye() + zed(); }\n",
    )
    .unwrap();
    fs::write(root.join("src/alpha/alpha.act"), "open api;\n").unwrap();
    fs::write(root.join("src/alpha/api.act"), "open verb aye() -> Int { return 1; }\n").unwrap();
    fs::write(root.join("src/zeta/zeta.act"), "open api;\n").unwrap();
    fs::write(root.join("src/zeta/api.act"), "open verb zed() -> Int { return 2; }\n").unwrap();
}

fn build_objects(root: &Path, input: &Path, output: &Path) {
    let result = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args(["build", input.to_str().unwrap(), "--strict", "--emit", "obj", "-o"])
        .arg(output)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(result.status.success(), "build stderr: {}", String::from_utf8_lossy(&result.stderr));
}

#[test]
fn repeated_multi_object_builds_have_identical_objects_and_symbol_manifests() {
    let root = std::env::temp_dir().join(format!("actus-reproducibility-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    write_fixture(&root);
    let input = root.join("src/main.act");
    let first = root.join("first.obj");
    let second = root.join("second.obj");
    build_objects(&root, &input, &first);
    build_objects(&root, &input, &second);

    assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
    assert_eq!(
        fs::read_to_string(root.join("first.symbols")).unwrap(),
        fs::read_to_string(root.join("second.symbols")).unwrap()
    );
    let manifest = fs::read_to_string(root.join("first.symbols")).unwrap();
    assert!(manifest.contains("actus_mod_5_alpha__verb_aye"));
    assert!(manifest.contains("actus_mod_4_zeta__verb_zed"));
    assert!(
        manifest.find("name = \"root\"").unwrap()
            < manifest.find("name = \"actus_mod_5_alpha\"").unwrap()
    );
    assert!(
        manifest.find("name = \"actus_mod_5_alpha\"").unwrap()
            < manifest.find("name = \"actus_mod_4_zeta\"").unwrap()
    );
    assert!(root.join("first.module.actus_mod_5_alpha.obj").is_file());
    assert!(root.join("first.module.actus_mod_4_zeta.obj").is_file());
    fs::remove_dir_all(root).unwrap();
}
