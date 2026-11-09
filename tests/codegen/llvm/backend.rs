#![cfg(feature = "llvm")]

use actus::ast::Role;
#[cfg(feature = "cranelift")]
use actus::codegen::cranelift::emit_contract_module_object;
use actus::codegen::link_objects;
use actus::codegen::llvm::{
    ObjectEmissionOptions, emit_module, emit_object,
    emit_object_bytes_with_configuration_and_float_policy,
    emit_object_with_configuration_and_options,
};
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendCaseMode, BackendCleanupArgument,
    BackendCleanupBridge, BackendCleanupOperand, BackendCleanupResult, BackendEdgeArgument,
    BackendEnumLayout, BackendExternal, BackendFailure, BackendFunction, BackendInstruction,
    BackendLayout, BackendModule, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendValueId, BackendValueType, LoweredCleanupInstruction, SymbolIdentity,
    SymbolKind, lower_scalar_program,
};
use actus::configuration::CompilerConfiguration;
#[cfg(feature = "cranelift")]
use actus::configuration::NativeBackendConfiguration;
use actus::lexer::scan;
use actus::parser::parse;
use object::{BinaryFormat, Object, ObjectSection, ObjectSymbol};

pub(crate) fn llvm_tool(name: &str) -> std::path::PathBuf {
    let config = std::path::PathBuf::from(env!(
        "ACTUS_LLVM_CONFIG_PATH",
        "Actus must expose the selected llvm-config path for LLVM tool tests"
    ));
    let version = std::process::Command::new(&config)
        .arg("--version")
        .output()
        .expect("selected llvm-config should run");
    assert!(version.status.success(), "selected llvm-config failed");
    let version = String::from_utf8(version.stdout).expect("llvm-config version should be UTF-8");
    assert_eq!(version.trim(), "23.1.2", "LLVM 23.1.2 is required for LLVM tool tests");
    let prefix = std::process::Command::new(&config)
        .arg("--prefix")
        .output()
        .expect("selected llvm-config should report its prefix");
    assert!(prefix.status.success(), "llvm-config --prefix failed");
    let selected_prefix = std::fs::canonicalize(
        String::from_utf8(prefix.stdout).expect("llvm-config prefix should be UTF-8").trim(),
    )
    .expect("selected llvm-config prefix should exist");
    let expected_prefix = std::env::var_os("LLVM_SYS_231_PREFIX")
        .or_else(|| option_env!("LLVM_SYS_231_PREFIX").map(std::ffi::OsString::from))
        .map(std::path::PathBuf::from)
        .map(|path| std::fs::canonicalize(path).expect("LLVM_SYS_231_PREFIX should exist"));
    if let Some(expected_prefix) = expected_prefix {
        assert_eq!(
            selected_prefix, expected_prefix,
            "llvm-config prefix does not match LLVM_SYS_231_PREFIX"
        );
    }
    let mut tool = selected_prefix.join("bin").join(name);
    if cfg!(windows) && tool.extension().is_none() {
        tool.set_extension("exe");
    }
    assert!(tool.is_file(), "LLVM 23.1.2 tool is missing: {}", tool.display());
    tool
}

fn symbol_name_matches(format: BinaryFormat, actual: &str, expected: &str) -> bool {
    actual == expected
        || (format == BinaryFormat::MachO && actual.strip_prefix('_') == Some(expected))
}

fn has_symbol<'data>(object: &object::File<'data>, expected: &str) -> bool {
    object
        .symbols()
        .filter_map(|symbol| symbol.name().ok())
        .any(|actual| symbol_name_matches(object.format(), actual, expected))
}

#[cfg(all(feature = "cranelift", target_os = "linux"))]
use actus::codegen::BackendCleanupResolvedDescriptor;

use std::fs;
use std::path::{Path, PathBuf};

mod bridge_contracts;
mod contracts;
mod ir;
mod objects;
mod regressions;
mod runtime;
mod stack_contracts;
