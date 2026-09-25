use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use actus::packaging::create_package_archive;
use actus::registry::{LocalRegistry, PackageCache, PackageIndex, PackageRecord, TrustPolicy};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("actus-registry-{stamp}"));
        fs::create_dir_all(root.join("src")).expect("create fixture root");
        Self { root }
    }

    fn write_project(&self) {
        fs::write(
            self.root.join("Arca.toml"),
            "[package]\nname = \"demo\"\nversion = \"1.2.3\"\nedition = \"alpha\"\n",
        )
        .expect("write manifest");
        fs::write(self.root.join("src/main.act"), "verb main() -> Int { return 0; }\n")
            .expect("write source");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn publishes_archive_to_index_and_checksum_cache() {
    let fixture = Fixture::new();
    fixture.write_project();
    let archive_path = fixture.root.join("demo.arca");
    let archive = create_package_archive(&fixture.root, &archive_path).expect("create archive");
    let registry_path = fixture.root.join("registry");
    let record = PackageRecord {
        name: "demo".to_owned(),
        version: "1.2.3".to_owned(),
        checksum: archive.checksum.clone(),
        archive: "demo-1.2.3.arca".to_owned(),
    };
    let registry = LocalRegistry::open(&registry_path);
    let published = registry
        .publish(&archive_path, record.clone(), &TrustPolicy::integrity_only())
        .expect("publish archive");
    let cached = PackageCache::new(fixture.root.join("cache"))
        .store(&archive_path, &archive.checksum)
        .expect("cache archive");
    assert!(published.is_file());
    assert!(cached.is_file());
    let index = PackageIndex::load(&registry_path.join("index.toml")).expect("read index");
    assert_eq!(index.packages, vec![record]);
}

#[test]
fn rejects_untrusted_checksum_under_strict_policy() {
    let fixture = Fixture::new();
    fixture.write_project();
    let archive_path = fixture.root.join("demo.arca");
    let archive = create_package_archive(&fixture.root, &archive_path).expect("create archive");
    let trust = TrustPolicy::trusted_checksums(vec!["different-checksum".to_owned()]);
    let record = PackageRecord {
        name: "demo".to_owned(),
        version: "1.2.3".to_owned(),
        checksum: archive.checksum,
        archive: "demo-1.2.3.arca".to_owned(),
    };
    let error = LocalRegistry::open(fixture.root.join("registry"))
        .publish(&archive_path, record, &trust)
        .expect_err("untrusted checksum must fail");
    assert!(error.to_string().contains("not trusted"));
}

#[cfg(unix)]
#[test]
fn publish_command_validates_packages_and_updates_local_registry() {
    let fixture = Fixture::new();
    fixture.write_project();
    let registry = fixture.root.join("registry");
    let cache = fixture.root.join("cache");
    let output = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "publish",
            "--registry",
            registry.to_str().unwrap(),
            "--cache",
            cache.to_str().unwrap(),
        ])
        .current_dir(&fixture.root)
        .output()
        .expect("run publish command");
    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(registry.join("index.toml").is_file());
    assert_eq!(fs::read_dir(cache).unwrap().count(), 1);
    assert!(String::from_utf8_lossy(&output.stdout).contains("published demo@1.2.3"));
}
