use serde::Serialize;

const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ToolchainIdentity {
    pub schema_version: u32,
    pub actus_version: String,
    pub commit: String,
    pub llvm: LlvmIdentity,
    pub host: String,
    pub backends: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LlvmIdentity {
    pub version: String,
    pub status: String,
    pub release_tag: Option<String>,
    pub archive_sha256: Option<String>,
    pub source_revision: Option<String>,
}

pub fn toolchain_identity() -> ToolchainIdentity {
    ToolchainIdentity {
        schema_version: SCHEMA_VERSION,
        actus_version: env!("CARGO_PKG_VERSION").to_owned(),
        commit: option_env!("ACTUS_GIT_COMMIT").unwrap_or("unknown").to_owned(),
        llvm: LlvmIdentity {
            version: option_env!("ACTUS_LLVM_VERSION").unwrap_or("unknown").to_owned(),
            status: option_env!("ACTUS_LLVM_STATUS").unwrap_or("unknown").to_owned(),
            release_tag: optional_identity_value(option_env!("ACTUS_LLVM_RELEASE_TAG")),
            archive_sha256: optional_identity_value(option_env!("ACTUS_LLVM_ARCHIVE_SHA256")),
            source_revision: optional_identity_value(option_env!("ACTUS_LLVM_SOURCE_REVISION")),
        },
        host: option_env!("ACTUS_HOST_TRIPLE").unwrap_or("unknown").to_owned(),
        backends: vec!["llvm".to_owned(), "cranelift".to_owned()],
    }
}

pub fn strict_identity_error() -> Option<String> {
    if option_env!("ACTUS_LLVM_STATUS") != Some("unverified") {
        return None;
    }
    let prefix = std::env::var("LLVM_SYS_231_PREFIX")
        .unwrap_or_else(|_| ".actus-toolchain/llvm-23.1.2".to_owned());
    Some(format!(
        "error[E1806]: strict LLVM build requires a verified toolchain; selected prefix {prefix} is unverified; run scripts/bootstrap_llvm.sh or scripts/bootstrap_llvm.ps1"
    ))
}

fn optional_identity_value(value: Option<&'static str>) -> Option<String> {
    value.filter(|value| !value.is_empty()).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::toolchain_identity;

    #[test]
    fn identity_has_a_stable_schema_and_no_absolute_path() {
        let identity = toolchain_identity();
        let json = serde_json::to_string(&identity).expect("identity should serialize");
        assert_eq!(identity.schema_version, 1);
        assert!(!json.contains("/home/"));
        assert!(!json.contains("\\\\"));
    }
}
