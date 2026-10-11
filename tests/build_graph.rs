#[cfg(feature = "llvm")]
#[test]
fn backend_change_invalidates_generated_artifacts() {
    use std::fs;

    use actus::build_graph::{invalidate_stale_artifact, metadata_path, write_metadata};
    use actus::configuration::CompilerConfiguration;

    let root =
        std::env::temp_dir().join(format!("actus-build-graph-backend-{}", std::process::id()));
    let cranelift_manifest = root.join("cranelift.toml");
    let llvm_manifest = root.join("llvm.toml");
    fs::create_dir_all(&root).expect("create metadata fixture root");
    fs::write(
        &cranelift_manifest,
        "[package]\nname = \"backend-metadata\"\nversion = \"0.1.0\"\n\n[build]\nbackend = \"cranelift\"\n",
    )
    .expect("write Cranelift manifest");
    fs::write(
        &llvm_manifest,
        "[package]\nname = \"backend-metadata\"\nversion = \"0.1.0\"\n\n[build]\nbackend = \"llvm\"\n",
    )
    .expect("write LLVM manifest");
    let cranelift = CompilerConfiguration::from_manifest(&cranelift_manifest)
        .expect("load Cranelift configuration");
    let llvm =
        CompilerConfiguration::from_manifest(&llvm_manifest).expect("load LLVM configuration");
    let artifact = cranelift.capsula_target_directory().join("backend.obj");
    fs::create_dir_all(artifact.parent().expect("artifact parent"))
        .expect("create artifact directory");
    fs::write(&artifact, b"cranelift-object").expect("write stale artifact");
    write_metadata(&artifact, &cranelift).expect("write Cranelift metadata");
    assert!(metadata_path(&artifact).is_file());
    invalidate_stale_artifact(&artifact, &llvm).expect("backend change should invalidate artifact");
    assert!(!artifact.exists());
    assert!(!metadata_path(&artifact).exists());
    let _ = fs::remove_dir_all(root);
}
