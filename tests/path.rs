use std::path::PathBuf;

use actus::modules::{ModuleResolver, analyze_module, exports_module};

#[test]
fn std_path_representation_module_exports_validated_storage_contracts() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src");
    let resolver = ModuleResolver::new(source_root);
    let exports = exports_module(&resolver, "path").expect("std path facade should resolve");
    assert_path_exports(&exports);
    analyze_module(&resolver, "path").expect("std path representation should be valid");
}

fn assert_path_exports(exports: &actus::modules::ModuleExports) {
    for (kind, name) in [
        ("struct", "Path"),
        ("enum", "PathPlatform"),
        ("enum", "PathError"),
        ("verb", "path_from_posix"),
        ("verb", "path_from_windows_utf16"),
        ("struct", "PathComponent"),
        ("struct", "PathComponents"),
        ("verb", "parent"),
        ("verb", "file_name"),
        ("verb", "file_stem"),
        ("verb", "extension"),
        ("verb", "components"),
        ("verb", "next_component"),
        ("enum", "PosixRoot"),
        ("verb", "posix_is_separator"),
        ("verb", "posix_root"),
        ("enum", "WindowsRoot"),
        ("verb", "windows_is_separator"),
        ("verb", "windows_root"),
        ("verb", "is_absolute"),
        ("verb", "is_relative"),
        ("verb", "has_root"),
        ("verb", "starts_with"),
        ("verb", "ends_with"),
        ("verb", "normalize"),
        ("verb", "join"),
        ("verb", "push"),
        ("verb", "reserve_path"),
        ("verb", "set_extension"),
        ("verb", "set_file_name"),
    ] {
        assert!(exports.contains(kind, name), "missing {kind} {name}");
    }
}

#[test]
fn std_path_platform_contracts_preserve_raw_separator_and_root_rules() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/path");
    let posix = std::fs::read_to_string(source_root.join("posix.act"))
        .expect("POSIX parser source should exist");
    let windows = std::fs::read_to_string(source_root.join("windows.act"))
        .expect("Windows parser source should exist");
    for marker in ["47", "///usr//bin", "foo/bar", ".."] {
        assert!(posix.contains(marker), "missing POSIX raw fixture marker {marker}");
    }
    for marker in ["47", "92", "C:foo", "C:\\\\", "server", "share"] {
        assert!(windows.contains(marker), "missing Windows raw fixture marker {marker}");
    }
}

#[test]
fn std_path_error_contract_names_storage_invariants() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/path");
    let source = std::fs::read_to_string(source_root.join("error.act"))
        .expect("path error source should exist");
    for variant in [
        "EmbeddedNull",
        "InvalidEncoding",
        "InvalidLength",
        "MissingTerminator",
        "CapacityExceeded",
        "UnsupportedPlatform",
    ] {
        assert!(source.contains(variant), "missing PathError variant {variant}");
    }
}
