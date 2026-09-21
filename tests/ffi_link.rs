#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use actus::codegen::{emit_program_object, link_object};
use actus::configuration::{CompilerConfiguration, LibraryKind};
use actus::lexer::scan;
use actus::parser::parse;

#[test]
fn links_static_and_shared_c_libraries_from_an_arca_manifest() {
    for kind in [LibraryKind::Static, LibraryKind::Shared] {
        link_fixture(kind);
    }
}

fn link_fixture(kind: LibraryKind) {
    let root = fixture_root(kind);
    fs::create_dir_all(&root).expect("fixture directory should be created");
    let fixture_source = Path::new("tests/fixtures/ffi/add.c");
    let c_source = root.join("add.c");
    fs::copy(fixture_source, &c_source).expect("C fixture should be copied");
    build_c_library(&root, &c_source, kind);
    write_manifest(&root, kind);

    let source = "unsafe extern \"C\" verb actus_fixture_add(erg left: Int, erg right: Int) -> Int; verb main() -> Int { erg left = 40; erg right = 2; return actus_fixture_add(left: left, right: right); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("fixture program should parse");
    let object_path = root.join("main.o");
    let executable = root.join("main");
    fs::write(&object_path, emit_program_object(&program, "main").expect("object should emit"))
        .expect("object should be written");
    let configuration = CompilerConfiguration::from_manifest(&root.join("Arca.toml"))
        .expect("fixture manifest should load");
    link_object(&object_path, &executable, &configuration).expect("C library should link");

    let mut command = Command::new(&executable);
    command.env("LD_LIBRARY_PATH", &root);
    let status = command.status().expect("linked fixture should run");
    assert_eq!(status.code(), Some(42));
    fs::remove_dir_all(&root).expect("fixture directory should be removed");
}

fn fixture_root(kind: LibraryKind) -> PathBuf {
    let suffix = match kind {
        LibraryKind::Static => "static",
        LibraryKind::Shared => "shared",
    };
    std::env::temp_dir().join(format!("actus-ffi-{suffix}-{}", std::process::id()))
}

fn build_c_library(root: &Path, source: &Path, kind: LibraryKind) {
    let object = root.join("add.o");
    run_command(Command::new("cc").args(["-c", source.to_str().unwrap(), "-o"]).arg(&object));
    match kind {
        LibraryKind::Static => {
            run_command(
                Command::new("ar").args(["crs"]).arg(root.join("libactus_fixture.a")).arg(object),
            );
        }
        LibraryKind::Shared => {
            run_command(
                Command::new("cc")
                    .args(["-shared", "-fPIC"])
                    .arg(source)
                    .arg("-o")
                    .arg(root.join("libactus_fixture.so")),
            );
        }
    }
}

fn write_manifest(root: &Path, kind: LibraryKind) {
    let kind = match kind {
        LibraryKind::Static => "static",
        LibraryKind::Shared => "shared",
    };
    let manifest = format!(
        "[package]\nname = \"ffi_fixture\"\nversion = \"0.1.0\"\nentry = \"main\"\n\n[build]\nlibrary_paths = [\".\"]\n\n[[build.libraries]]\nname = \"actus_fixture\"\nkind = \"{kind}\"\n"
    );
    fs::write(root.join("Arca.toml"), manifest).expect("fixture manifest should be written");
}

fn run_command(command: &mut Command) {
    let status = command.status().expect("native C tool should start");
    assert!(status.success(), "native C tool should succeed: {command:?}");
}
