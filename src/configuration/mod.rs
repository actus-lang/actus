use std::ffi::{OsStr, OsString};

const LINKER_ENVIRONMENT_VARIABLE: &str = "ACTUS_LINKER";
const DEFAULT_LINKER: &str = "cc";
const DEFAULT_RUN_ARTIFACT_PREFIX: &str = "actus-run";
const DEFAULT_NATIVE_MODULE_NAME: &str = "actus";

#[derive(Clone, Debug)]
pub struct NativeBackendConfiguration {
    module_name: String,
    position_independent: bool,
}

impl Default for NativeBackendConfiguration {
    fn default() -> Self {
        Self { module_name: DEFAULT_NATIVE_MODULE_NAME.to_owned(), position_independent: true }
    }
}

impl NativeBackendConfiguration {
    pub fn new(module_name: impl Into<String>, position_independent: bool) -> Self {
        Self { module_name: module_name.into(), position_independent }
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn position_independent(&self) -> bool {
        self.position_independent
    }
}

#[derive(Clone, Debug)]
pub struct CompilerConfiguration {
    linker: OsString,
    run_artifact_prefix: String,
    native_backend: NativeBackendConfiguration,
}

impl CompilerConfiguration {
    pub fn from_environment() -> Self {
        let linker = std::env::var_os(LINKER_ENVIRONMENT_VARIABLE)
            .unwrap_or_else(|| OsString::from(DEFAULT_LINKER));
        Self {
            linker,
            run_artifact_prefix: DEFAULT_RUN_ARTIFACT_PREFIX.to_owned(),
            native_backend: NativeBackendConfiguration::default(),
        }
    }

    pub fn linker(&self) -> &OsStr {
        &self.linker
    }

    pub fn run_artifact_prefix(&self) -> &str {
        &self.run_artifact_prefix
    }

    pub fn native_backend(&self) -> &NativeBackendConfiguration {
        &self.native_backend
    }
}
