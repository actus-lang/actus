use std::ffi::{OsStr, OsString};

const LINKER_ENVIRONMENT_VARIABLE: &str = "ACTUS_LINKER";
const DEFAULT_LINKER: &str = "cc";
const DEFAULT_RUN_ARTIFACT_PREFIX: &str = "actus-run";

#[derive(Clone, Debug)]
pub struct CompilerConfiguration {
    linker: OsString,
    run_artifact_prefix: String,
}

impl CompilerConfiguration {
    pub fn from_environment() -> Self {
        let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
            .unwrap_or_else(|| OsString::from(DEFAULT_LINKER));
        Self { linker, run_artifact_prefix: DEFAULT_RUN_ARTIFACT_PREFIX.to_owned() }
    }

    pub fn linker(&self) -> &OsStr {
        &self.linker
    }

    pub fn run_artifact_prefix(&self) -> &str {
        &self.run_artifact_prefix
    }
}
