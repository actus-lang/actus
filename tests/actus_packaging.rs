use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::configuration::{Version, VersionConstraint};
use actus::packaging::{create_package_archive, verify_archive_checksum};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-package-{stamp}"));
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("create parent");
        fs::write(path, contents).expect("write fixture file");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn resolves_basic_semver_constraints() {
    let version = Version::parse("1.4.2").expect("parse version");
    assert!(VersionConstraint::parse("=1.4.2").unwrap().matches(&version));
    assert!(VersionConstraint::parse("^1.2.0").unwrap().matches(&version));
    assert!(VersionConstraint::parse(">=1.4.0").unwrap().matches(&version));
    assert!(!VersionConstraint::parse("^2.0.0").unwrap().matches(&version));
}

#[test]
fn rejects_invalid_semver_constraints() {
    assert!(VersionConstraint::parse("~1.0.0").is_err());
    assert!(VersionConstraint::parse("^1.0").is_err());
}

#[test]
fn creates_deterministic_archive_without_build_outputs() {
    let fixture = Fixture::new();
    fixture.write("Actus.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    fixture.write("src/main.act", "verb main() -> Int { return 0; }\n");
    fixture.write("capsula/debug/host/object.o", "generated");
    fixture.write(".git/config", "metadata");
    let output = fixture.root.join("demo.actus");
    let archive = create_package_archive(&fixture.root, &output).expect("create archive");
    assert_eq!(archive.files, vec!["Actus.toml", "src/main.act"]);
    verify_archive_checksum(&output, &archive.checksum).expect("checksum validates");
}

#[test]
fn archive_checksum_detects_tampering() {
    let fixture = Fixture::new();
    fixture.write("src/main.act", "verb main() -> Int { return 0; }\n");
    let output = fixture.root.join("demo.actus");
    let archive = create_package_archive(&fixture.root, &output).expect("create archive");
    fs::write(&output, b"tampered").expect("tamper archive");
    assert!(verify_archive_checksum(&output, &archive.checksum).is_err());
}
