use std::fs;
use std::path::{Path, PathBuf};

use actus::cli::run_with_args;

fn library_file(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src").join(relative);
    fs::read_to_string(path).expect("standard library source should exist")
}

fn copy_path_library(root: &Path) {
    for relative in [
        "lib.act",
        "path/path.act",
        "path/types.act",
        "path/storage.act",
        "path/error.act",
        "path/components.act",
        "path/posix.act",
        "path/windows.act",
        "path/predicates.act",
        "path/normalize.act",
        "path/builders.act",
    ] {
        let destination = root.join("src").join(relative);
        fs::create_dir_all(destination.parent().expect("path library parent"))
            .expect("create path library directory");
        fs::write(destination, library_file(relative)).expect("copy path library source");
    }
}

#[test]
fn std_path_file_name_executes_through_the_runtime_bridge() {
    let root = std::env::temp_dir().join(format!("actus-path-native-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).expect("create fixture root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"path-fixture\"\nversion = \"0.1.0\"\nsource_root = \"src\"\n",
    )
    .expect("write fixture manifest");
    copy_path_library(&root);
    fs::write(root.join("src/main.act"), path_fixture_source()).expect("write path fixture");
    let output = root.join("path-fixture");
    let status = run_with_args(
        [
            "build",
            root.join("src/main.act").to_str().expect("fixture path"),
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("fixture output"),
        ]
        .into_iter()
        .map(str::to_owned),
    );
    assert_eq!(status, 0);
    let execution = std::process::Command::new(&output).status().expect("run path fixture");
    assert_eq!(execution.code(), Some(0));
    fs::remove_dir_all(root).expect("remove path fixture");
}

fn path_fixture_source() -> &'static str {
    r#"
import path;

verb main() -> Int {
    erg raw = Buffer[0];
    append(raw, 47);
    append(raw, 97);
    append(raw, 98);
    append(raw, 99);
    append(raw, 46);
    append(raw, 116);
    append(raw, 120);
    append(raw, 116);
    append(raw, 0);
    erg result = path_from_posix(storage: dat raw);
    return case dat result {
        Result.Err(_) => 1,
        Result.Ok(value) => {
            erg path: Path = value;
            erg found = file_name(self: abs path);
            return case dat found {
                Option.Some(component) => case component.length {
                    7 => 0,
                    _ => 2,
                },
                Option.None => 3,
            };
        },
    };
}
"#
}
