use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

fn main() {
    println!("cargo:rerun-if-changed=src/runtime");
    println!("cargo:rerun-if-changed=.cargo/config.toml");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_231_PREFIX");
    println!("cargo:rustc-check-cfg=cfg(coverage)");
    println!("cargo:rustc-check-cfg=cfg(actus_test_faults)");

    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"));
    let archive = output_dir.join("libactus_runtime.a");
    let rustc = env::var_os("RUSTC").expect("Cargo must provide RUSTC");
    let mut command = Command::new(rustc);
    command
        .args(["--edition", "2024", "--crate-type", "staticlib"])
        .arg("--crate-name")
        .arg("actus_runtime")
        .arg("src/runtime/mod.rs");
    if env::var_os("CARGO_FEATURE_TEST_FAULT_INJECTION").is_some() {
        command.args(["--cfg", "actus_test_faults"]);
    }
    let status = command
        .arg("-o")
        .arg(&archive)
        .status()
        .expect("failed to invoke rustc for the native runtime");
    assert!(status.success(), "native runtime compilation failed");
    println!("cargo:rustc-env=ACTUS_RUNTIME_ARCHIVE={}", archive.display());

    if env::var_os("CARGO_FEATURE_LLVM").is_some() {
        let identity = validate_llvm_selection();
        emit_identity(&identity);
        build_llvm_target_options_shim();
    } else {
        emit_identity(&LlvmBuildIdentity::not_built());
    }
}

#[derive(Deserialize)]
struct BundledToolchainManifest {
    llvm_version: String,
    source_revision: String,
    host_triple: Option<String>,
}

#[derive(Deserialize)]
struct ArchiveManifest {
    sha256: String,
}

#[derive(Deserialize)]
struct ReleaseManifest {
    release_tag: String,
    archives: std::collections::BTreeMap<String, ArchiveManifest>,
}

struct LlvmBuildIdentity {
    version: String,
    status: &'static str,
    release_tag: String,
    archive_sha256: String,
    source_revision: String,
    host_triple: String,
}

impl LlvmBuildIdentity {
    fn not_built() -> Self {
        Self {
            version: "not-built".to_owned(),
            status: "not-built",
            release_tag: String::new(),
            archive_sha256: String::new(),
            source_revision: String::new(),
            host_triple: env::var("HOST").unwrap_or_else(|_| "unknown".to_owned()),
        }
    }
}

fn validate_llvm_selection() -> LlvmBuildIdentity {
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bundled_prefix = repository_root.join(".actus-toolchain/llvm-23.1.2");
    let configured_prefix = env::var_os("LLVM_SYS_231_PREFIX").map(PathBuf::from);
    let selected_prefix = configured_prefix
        .clone()
        .map(|prefix| if prefix.is_absolute() { prefix } else { repository_root.join(prefix) })
        .unwrap_or_else(|| bundled_prefix.clone());

    let bundled_identity = bundled_prefix.canonicalize().unwrap_or_else(|_| bundled_prefix.clone());
    let selected_identity =
        selected_prefix.canonicalize().unwrap_or_else(|_| selected_prefix.clone());
    if selected_identity == bundled_identity {
        if !selected_prefix.is_dir() {
            panic!(
                "the verified LLVM toolchain is missing at `{}`; run scripts/bootstrap_llvm.sh or scripts/bootstrap_llvm.ps1",
                selected_prefix.display()
            );
        }
        let bundled_manifest = selected_prefix.join("share/actus/toolchain.json");
        let bundled_manifest: BundledToolchainManifest = read_json(&bundled_manifest);
        let host_triple = bundled_manifest
            .host_triple
            .clone()
            .unwrap_or_else(|| env::var("HOST").unwrap_or_else(|_| "unknown".to_owned()));
        let release_manifest_path = repository_root.join("toolchains/llvm/llvm-23.1.2.json");
        let release_manifest: ReleaseManifest = read_json(&release_manifest_path);
        let archive = release_manifest.archives.get(&host_triple).unwrap_or_else(|| {
            panic!("LLVM release manifest has no archive for bundled host `{host_triple}`")
        });
        assert_eq!(
            bundled_manifest.llvm_version, "23.1.2",
            "bundled LLVM manifest has an unsupported version"
        );
        return LlvmBuildIdentity {
            version: bundled_manifest.llvm_version,
            status: "verified",
            release_tag: release_manifest.release_tag,
            archive_sha256: archive.sha256.clone(),
            source_revision: bundled_manifest.source_revision,
            host_triple,
        };
    }

    let llvm_config_name = if cfg!(windows) { "bin/llvm-config.exe" } else { "bin/llvm-config" };
    let llvm_config = selected_prefix.join(llvm_config_name);
    if !llvm_config.is_file() {
        panic!(
            "LLVM_SYS_231_PREFIX points to `{}`, but `{}` is missing",
            selected_prefix.display(),
            llvm_config.display()
        );
    }
    let version = command_output(&llvm_config, "--version");
    println!(
        "cargo:warning=LLVM toolchain override is unverified: prefix `{}`, version `{}`",
        selected_prefix.display(),
        version.trim()
    );
    LlvmBuildIdentity {
        version: version.trim().to_owned(),
        status: "unverified",
        release_tag: String::new(),
        archive_sha256: String::new(),
        source_revision: String::new(),
        host_triple: env::var("HOST").unwrap_or_else(|_| "unknown".to_owned()),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &std::path::Path) -> T {
    let contents = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", path.display()));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("invalid JSON in `{}`: {error}", path.display()))
}

fn emit_identity(identity: &LlvmBuildIdentity) {
    println!("cargo:rustc-env=ACTUS_LLVM_VERSION={}", identity.version);
    println!("cargo:rustc-env=ACTUS_LLVM_STATUS={}", identity.status);
    println!("cargo:rustc-env=ACTUS_LLVM_RELEASE_TAG={}", identity.release_tag);
    println!("cargo:rustc-env=ACTUS_LLVM_ARCHIVE_SHA256={}", identity.archive_sha256);
    println!("cargo:rustc-env=ACTUS_LLVM_SOURCE_REVISION={}", identity.source_revision);
    println!("cargo:rustc-env=ACTUS_HOST_TRIPLE={}", identity.host_triple);
    println!("cargo:rustc-env=ACTUS_GIT_COMMIT={}", git_commit());
    println!("cargo:rerun-if-changed=toolchains/llvm/llvm-23.1.2.json");
}

fn git_commit() -> String {
    Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|commit| commit.trim().to_owned())
        .filter(|commit| !commit.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn build_llvm_target_options_shim() {
    let llvm_config = env::var_os("DEP_LLVM_23_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("LLVM feature enabled but llvm-sys did not provide llvm-config"));
    println!("cargo:rustc-env=ACTUS_LLVM_CONFIG_PATH={}", llvm_config.display());
    let version = command_output(&llvm_config, "--version");
    let version = version.split_whitespace().next().unwrap_or_default();
    let mut components = version.split('.');
    let compatible = components.next() == Some("23") && components.next() == Some("1");
    assert!(
        compatible,
        "LLVM target-options shim requires LLVM 23.1, found `{version}` from `{}`",
        llvm_config.display()
    );
    let cxxflags = command_output(&llvm_config, "--cxxflags");
    let mut build = cc::Build::new();
    build.cpp(true).file("src/codegen/llvm/shim/target_options.cpp");
    for flag in cxxflags.split_whitespace() {
        build.flag(flag);
    }
    build.compile("actus_llvm_target_options");
    println!("cargo:rerun-if-changed=src/codegen/llvm/shim/target_options.cpp");
}

fn command_output(program: &std::path::Path, argument: &str) -> String {
    let output = Command::new(program)
        .arg(argument)
        .output()
        .unwrap_or_else(|error| panic!("failed to run `{}`: {error}", program.display()));
    assert!(
        output.status.success(),
        "`{} {argument}` failed: {}",
        program.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap_or_else(|error| {
        panic!("`{} {argument}` returned non-UTF-8 output: {error}", program.display())
    })
}
