use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::configuration::{ArcaLock, LockedPackage};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-lock-{stamp}"));
        fs::create_dir_all(&root).expect("create fixture root");
        Self { root }
    }

    fn manifest(&self, dependencies: &str) -> PathBuf {
        let path = self.root.join("Arca.toml");
        fs::write(
            &path,
            format!(
                "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n\n[package.dependencies]\n{dependencies}"
            ),
        )
        .expect("write manifest");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn serializes_packages_in_deterministic_order() {
    let lock = ArcaLock::from_packages(vec![
        LockedPackage {
            name: "zeta".to_owned(),
            version: "1.0.0".to_owned(),
            source: Some("registry".to_owned()),
            path: None,
            checksum: None,
        },
        LockedPackage {
            name: "alpha".to_owned(),
            version: "2.0.0".to_owned(),
            source: Some("registry".to_owned()),
            path: None,
            checksum: Some("abc".to_owned()),
        },
    ]);
    let serialized = lock.serialize().expect("lockfile should serialize");
    assert!(
        serialized.find("name = \"alpha\"").unwrap() < serialized.find("name = \"zeta\"").unwrap()
    );
    assert_eq!(serialized, lock.serialize().expect("serialization is deterministic"));
}

#[test]
fn generates_and_validates_lockfile_against_manifest_dependencies() {
    let fixture = Fixture::new();
    let manifest = fixture.manifest("zeta = \"1.0.0\"\nalpha = \"2.0.0\"\n");
    let lock = ArcaLock::generate_from_manifest(&manifest).expect("lockfile should generate");
    lock.validate_against_manifest(&manifest).expect("generated lockfile should validate");
    let path = ArcaLock::sync(&manifest).expect("lockfile should be written");
    assert_eq!(path, fixture.root.join("Arca.lock"));
    let source = fs::read_to_string(path).expect("read generated lockfile");
    assert_eq!(ArcaLock::parse(&source).expect("parse generated lockfile"), lock);
}

#[test]
fn rejects_lockfile_that_does_not_match_manifest() {
    let fixture = Fixture::new();
    let manifest = fixture.manifest("alpha = \"2.0.0\"\n");
    let stale = ArcaLock::from_packages(vec![LockedPackage {
        name: "alpha".to_owned(),
        version: "1.0.0".to_owned(),
        source: Some("registry".to_owned()),
        path: None,
        checksum: None,
    }]);
    stale.validate_against_manifest(&manifest).expect_err("stale lockfile must fail");
    assert!(
        stale
            .validate_against_manifest(&manifest)
            .expect_err("stale lockfile must fail")
            .to_string()
            .contains("LockfileOutOfDate")
    );
}
