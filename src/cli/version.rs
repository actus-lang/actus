use serde::Serialize;

pub(super) fn version_command(arguments: impl Iterator<Item = String>) -> i32 {
    let mut json = false;
    for argument in arguments {
        if argument == "--json" {
            if json {
                eprintln!("error: duplicate --json option");
                return 2;
            }
            json = true;
        } else {
            eprintln!("error: unexpected argument {argument}");
            return 2;
        }
    }
    let identity = identity();
    if json {
        match serde_json::to_string_pretty(&identity) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("error: cannot serialize toolchain identity: {error}");
                return 1;
            }
        }
    } else {
        println!("actus {} ({})", identity.actus_version, identity.commit);
        println!(
            "llvm {} {}{}",
            identity.llvm.version,
            identity.llvm.status,
            identity
                .llvm
                .release_tag
                .as_deref()
                .map(|tag| format!(
                    " {tag} sha256:{}",
                    identity.llvm.archive_sha256.as_deref().unwrap_or("")
                ))
                .unwrap_or_default()
        );
        println!("host {}", identity.host);
        println!("backends {}", identity.backends.join(", "));
    }
    0
}

#[derive(Serialize)]
struct VersionIdentity {
    schema_version: u32,
    actus_version: String,
    commit: String,
    llvm: LlvmVersionIdentity,
    host: String,
    backends: Vec<String>,
}

#[derive(Serialize)]
struct LlvmVersionIdentity {
    version: String,
    status: String,
    release_tag: Option<String>,
    archive_sha256: Option<String>,
    source_revision: Option<String>,
}

fn identity() -> VersionIdentity {
    #[cfg(feature = "llvm")]
    {
        let identity = crate::codegen::llvm::toolchain_identity();
        return VersionIdentity {
            schema_version: identity.schema_version,
            actus_version: identity.actus_version,
            commit: identity.commit,
            llvm: LlvmVersionIdentity {
                version: identity.llvm.version,
                status: identity.llvm.status,
                release_tag: identity.llvm.release_tag,
                archive_sha256: identity.llvm.archive_sha256,
                source_revision: identity.llvm.source_revision,
            },
            host: identity.host,
            backends: identity.backends,
        };
    }

    #[cfg(not(feature = "llvm"))]
    VersionIdentity {
        schema_version: 1,
        actus_version: env!("CARGO_PKG_VERSION").to_owned(),
        commit: option_env!("ACTUS_GIT_COMMIT").unwrap_or("unknown").to_owned(),
        llvm: LlvmVersionIdentity {
            version: "not-built".to_owned(),
            status: "not-built".to_owned(),
            release_tag: None,
            archive_sha256: None,
            source_revision: None,
        },
        host: option_env!("ACTUS_HOST_TRIPLE").unwrap_or("unknown").to_owned(),
        backends: vec!["cranelift".to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::identity;

    #[test]
    fn identity_is_json_serializable() {
        let json = serde_json::to_string(&identity()).expect("identity should serialize");
        assert!(json.contains("schema_version"));
        assert!(json.contains("actus_version"));
        assert!(json.contains("backends"));
    }
}
