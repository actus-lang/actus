use std::fs;
use std::path::PathBuf;

pub(super) fn library_source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/io").join(file);
    fs::read_to_string(path).expect("standard library source should exist")
}
