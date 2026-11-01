#![cfg(feature = "llvm")]

use actus::ast::Role;
#[cfg(feature = "cranelift")]
use actus::codegen::cranelift::emit_contract_module_object;
#[cfg(target_os = "linux")]
use actus::codegen::link_objects;
use actus::codegen::llvm::{
    emit_module, emit_object, emit_object_bytes_with_configuration_and_float_policy,
};
use actus::codegen::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendCaseMode, BackendCleanupArgument,
    BackendCleanupBridge, BackendCleanupOperand, BackendCleanupResult, BackendEdgeArgument,
    BackendEnumLayout, BackendExternal, BackendFailure, BackendFunction, BackendInstruction,
    BackendLayout, BackendModule, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendValueId, BackendValueType, LoweredCleanupInstruction, SymbolIdentity,
    SymbolKind, lower_scalar_program,
};
#[cfg(target_os = "linux")]
use actus::configuration::CompilerConfiguration;
#[cfg(feature = "cranelift")]
use actus::configuration::NativeBackendConfiguration;
use actus::lexer::scan;
use actus::parser::parse;
use object::{BinaryFormat, Object, ObjectSection, ObjectSymbol};

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

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

mod bridge_contracts;
mod contracts;
mod ir;
mod objects;
mod regressions;
mod runtime;
