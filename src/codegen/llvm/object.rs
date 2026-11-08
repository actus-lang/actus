use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::Once;

use ::object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};
use llvm_sys::core::{
    LLVMDisposeMemoryBuffer, LLVMDisposeMessage, LLVMGetBufferSize, LLVMGetBufferStart,
    LLVMSetTarget,
};
use llvm_sys::target_machine::{
    LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetDataLayout,
    LLVMCreateTargetMachine, LLVMDisposeTargetMachine, LLVMGetTargetFromTriple, LLVMRelocMode,
    LLVMTargetHasTargetMachine, LLVMTargetMachineEmitToFile, LLVMTargetMachineEmitToMemoryBuffer,
};
use target_lexicon::BinaryFormat;

use super::super::BackendEmitError;
use super::emission::{ObjectEmissionOptions, ObjectOptimizationLevel};
use crate::target::TargetSpec;

static LLVM_TARGETS_INITIALIZED: Once = Once::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackSizeRecord {
    pub symbol: String,
    pub address: u64,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackSizeParseError {
    InvalidUleb128,
    Uleb128Overflow,
    TruncatedRecord { offset: usize },
    MissingRelocation { offset: usize },
    InvalidRelocationWidth { offset: usize, bits: u8 },
    UnknownSymbol { offset: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StackSizeRelocation {
    offset: usize,
    width: usize,
    symbol: String,
    address: u64,
}

pub fn parse_stack_sizes<'data>(
    file: &::object::File<'data>,
) -> Result<Vec<StackSizeRecord>, StackSizeParseError> {
    file.sections().filter(|section| section.name_bytes().ok() == Some(b".stack_sizes")).try_fold(
        Vec::new(),
        |mut records, section| {
            records.extend(parse_stack_size_section(file, &section)?);
            Ok(records)
        },
    )
}

fn parse_stack_size_section<'data>(
    file: &::object::File<'data>,
    section: &::object::Section<'data, 'data>,
) -> Result<Vec<StackSizeRecord>, StackSizeParseError> {
    let data = section.data().map_err(|_| StackSizeParseError::InvalidUleb128)?;
    let relocations = section
        .relocations()
        .map(|(offset, relocation)| {
            let (symbol, address) = resolve_stack_relocation(
                file,
                relocation.target(),
                relocation.addend(),
                offset as usize,
            )?;
            let width = relocation.size() / 8;
            if !matches!(width, 4 | 8) {
                return Err(StackSizeParseError::InvalidRelocationWidth {
                    offset: offset as usize,
                    bits: relocation.size(),
                });
            }
            Ok(StackSizeRelocation {
                offset: offset as usize,
                width: width as usize,
                symbol,
                address,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    parse_stack_size_records(data, &relocations)
}

fn resolve_stack_relocation<'data>(
    file: &::object::File<'data>,
    target: RelocationTarget,
    addend: i64,
    offset: usize,
) -> Result<(String, u64), StackSizeParseError> {
    let symbol = match target {
        RelocationTarget::Symbol(index) => file.symbol_by_index(index).ok(),
        RelocationTarget::Section(index) => file
            .symbols()
            .find(|symbol| symbol.section_index() == Some(index) && symbol.is_definition()),
        _ => None,
    };
    let symbol = symbol.ok_or(StackSizeParseError::UnknownSymbol { offset })?;
    let address =
        u64::try_from(addend).ok().and_then(|addend| symbol.address().checked_add(addend));
    let symbol = if symbol.kind() == ::object::SymbolKind::Section {
        file.symbols().find(|candidate| {
            candidate.section_index() == symbol.section_index()
                && candidate.is_definition()
                && address == Some(candidate.address())
        })
    } else {
        Some(symbol)
    };
    let symbol = symbol.ok_or(StackSizeParseError::UnknownSymbol { offset })?;
    let name = symbol.name().ok().filter(|name| !name.is_empty());
    let name = name.ok_or(StackSizeParseError::UnknownSymbol { offset })?;
    Ok((name.to_owned(), symbol.address()))
}

fn parse_stack_size_records(
    data: &[u8],
    relocations: &[StackSizeRelocation],
) -> Result<Vec<StackSizeRecord>, StackSizeParseError> {
    let mut records = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let record_offset = offset;
        let relocation = relocations
            .iter()
            .find(|relocation| relocation.offset == offset)
            .ok_or(StackSizeParseError::MissingRelocation { offset })?;
        if relocation.symbol.is_empty() {
            return Err(StackSizeParseError::UnknownSymbol { offset });
        }
        let size_offset = offset
            .checked_add(relocation.width)
            .ok_or(StackSizeParseError::TruncatedRecord { offset: record_offset })?;
        if size_offset > data.len() {
            return Err(StackSizeParseError::TruncatedRecord { offset: record_offset });
        }
        let (size, end) = read_uleb128(data, size_offset)?;
        records.push(StackSizeRecord {
            symbol: relocation.symbol.clone(),
            address: relocation.address,
            size,
        });
        offset = end;
    }
    Ok(records)
}

fn read_uleb128(data: &[u8], mut offset: usize) -> Result<(u64, usize), StackSizeParseError> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while offset < data.len() {
        let byte = data[offset];
        offset += 1;
        if shift >= 64 || (shift == 63 && byte > 1) {
            return Err(StackSizeParseError::Uleb128Overflow);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, offset));
        }
        shift += 7;
    }
    Err(StackSizeParseError::TruncatedRecord { offset })
}

#[cfg(test)]
mod stack_size_tests {
    use super::*;

    fn relocation(offset: usize, width: usize, symbol: &str, address: u64) -> StackSizeRelocation {
        StackSizeRelocation { offset, width, symbol: symbol.to_owned(), address }
    }

    #[test]
    fn empty_section_has_no_records() {
        assert_eq!(parse_stack_size_records(&[], &[]), Ok(Vec::new()));
    }

    #[test]
    fn parses_multiple_uleb128_records_and_symbols() {
        let data = [0, 0, 0, 0, 0x81, 0x01, 0, 0, 0, 0, 0x7f];
        let relocations = [relocation(0, 4, "first", 0x1000), relocation(6, 4, "second", 0x2000)];
        assert_eq!(
            parse_stack_size_records(&data, &relocations),
            Ok(vec![
                StackSizeRecord { symbol: "first".to_owned(), address: 0x1000, size: 129 },
                StackSizeRecord { symbol: "second".to_owned(), address: 0x2000, size: 127 },
            ])
        );
    }

    #[test]
    fn rejects_uleb128_overflow() {
        let data = [0, 0, 0, 0, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02];
        let relocations = [relocation(0, 4, "overflow", 0x1000)];
        assert_eq!(
            parse_stack_size_records(&data, &relocations),
            Err(StackSizeParseError::Uleb128Overflow)
        );
    }

    #[test]
    fn rejects_a_record_truncated_after_relocation() {
        let data = [0, 0, 0, 0, 0x80];
        let relocations = [relocation(0, 4, "truncated", 0x1000)];
        assert_eq!(
            parse_stack_size_records(&data, &relocations),
            Err(StackSizeParseError::TruncatedRecord { offset: 5 })
        );
    }

    #[test]
    fn rejects_unknown_symbol() {
        let data = [0, 0, 0, 0, 0x01];
        let relocations =
            [StackSizeRelocation { offset: 0, width: 4, symbol: String::new(), address: 0 }];
        assert_eq!(
            parse_stack_size_records(&data, &relocations),
            Err(StackSizeParseError::UnknownSymbol { offset: 0 })
        );
    }
}

pub(super) fn emit_object(
    module: llvm_sys::prelude::LLVMModuleRef,
    target: &TargetSpec,
    output: &Path,
    position_independent: bool,
    options: ObjectEmissionOptions,
) -> Result<(), BackendEmitError> {
    initialize_targets()?;
    let triple = CString::new(target.triple().to_string())
        .map_err(|_| BackendEmitError("LLVM target triple contains NUL".to_owned()))?;
    let output = output
        .to_str()
        .ok_or_else(|| BackendEmitError("LLVM object path is not valid UTF-8".to_owned()))?;
    let output = CString::new(output)
        .map_err(|_| BackendEmitError("LLVM object path contains NUL".to_owned()))?;
    let machine = TargetMachine::new(
        triple.as_c_str(),
        position_independent,
        target.object_format == BinaryFormat::Elf,
        options,
    )?;
    let data_layout = unsafe { LLVMCreateTargetDataLayout(machine.raw) };
    if data_layout.is_null() {
        return Err(BackendEmitError("LLVM failed to create target data layout".to_owned()));
    }
    unsafe { LLVMSetTarget(module, triple.as_ptr()) };
    unsafe { llvm_sys::target::LLVMSetModuleDataLayout(module, data_layout) };
    let mut error = ptr::null_mut();
    let failed = unsafe {
        LLVMTargetMachineEmitToFile(
            machine.raw,
            module,
            output.as_ptr().cast_mut(),
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut error,
        ) != 0
    };
    unsafe { llvm_sys::target::LLVMDisposeTargetData(data_layout) };
    if failed {
        return Err(BackendEmitError(format_llvm_error(error, "LLVM object emission failed")));
    }
    Ok(())
}

pub(super) fn emit_object_bytes(
    module: llvm_sys::prelude::LLVMModuleRef,
    target: &TargetSpec,
    position_independent: bool,
    options: ObjectEmissionOptions,
) -> Result<Vec<u8>, BackendEmitError> {
    initialize_targets()?;
    let triple = CString::new(target.triple().to_string())
        .map_err(|_| BackendEmitError("LLVM target triple contains NUL".to_owned()))?;
    let machine = TargetMachine::new(
        triple.as_c_str(),
        position_independent,
        target.object_format == BinaryFormat::Elf,
        options,
    )?;
    let data_layout = unsafe { LLVMCreateTargetDataLayout(machine.raw) };
    if data_layout.is_null() {
        return Err(BackendEmitError("LLVM failed to create target data layout".to_owned()));
    }
    unsafe { LLVMSetTarget(module, triple.as_ptr()) };
    unsafe { llvm_sys::target::LLVMSetModuleDataLayout(module, data_layout) };
    let mut error = ptr::null_mut();
    let mut buffer = ptr::null_mut();
    let failed = unsafe {
        LLVMTargetMachineEmitToMemoryBuffer(
            machine.raw,
            module,
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut error,
            &mut buffer,
        ) != 0
    };
    unsafe { llvm_sys::target::LLVMDisposeTargetData(data_layout) };
    if failed {
        if !buffer.is_null() {
            unsafe { LLVMDisposeMemoryBuffer(buffer) };
        }
        return Err(BackendEmitError(format_llvm_error(error, "LLVM object emission failed")));
    }
    if buffer.is_null() {
        return Err(BackendEmitError("LLVM returned no object memory buffer".to_owned()));
    }
    let size = unsafe { LLVMGetBufferSize(buffer) };
    let start = unsafe { LLVMGetBufferStart(buffer) };
    if size != 0 && start.is_null() {
        unsafe { LLVMDisposeMemoryBuffer(buffer) };
        return Err(BackendEmitError("LLVM returned an invalid object memory buffer".to_owned()));
    }
    let bytes = unsafe { std::slice::from_raw_parts(start.cast::<u8>(), size).to_vec() };
    unsafe { LLVMDisposeMemoryBuffer(buffer) };
    Ok(bytes)
}

fn initialize_targets() -> Result<(), BackendEmitError> {
    LLVM_TARGETS_INITIALIZED.call_once(|| {
        unsafe {
            llvm_sys::target::LLVM_InitializeAllTargetInfos();
            llvm_sys::target::LLVM_InitializeAllTargets();
            llvm_sys::target::LLVM_InitializeAllTargetMCs();
            llvm_sys::target::LLVM_InitializeAllAsmPrinters();
        }
        // LLVM exposes this control as a process-global cl::opt rather than a
        // TargetMachine option. Set it before any TargetMachine is created so
        // every LLVM emission in this process has the same frame accounting.
        super::shim::disable_x86_call_frame_optimization();
    });
    Ok(())
}

struct TargetMachine {
    raw: llvm_sys::target_machine::LLVMTargetMachineRef,
}

impl TargetMachine {
    fn new(
        triple: &std::ffi::CStr,
        position_independent: bool,
        is_elf: bool,
        options: super::emission::ObjectEmissionOptions,
    ) -> Result<Self, BackendEmitError> {
        let mut llvm_target = ptr::null_mut();
        let mut error = ptr::null_mut();
        let failed =
            unsafe { LLVMGetTargetFromTriple(triple.as_ptr(), &mut llvm_target, &mut error) != 0 };
        if failed {
            return Err(BackendEmitError(format_llvm_error(error, "LLVM target lookup failed")));
        }
        if llvm_target.is_null() || unsafe { LLVMTargetHasTargetMachine(llvm_target) == 0 } {
            return Err(BackendEmitError("LLVM target has no target machine".to_owned()));
        }
        let cpu = CString::new("").expect("empty CPU string cannot contain NUL");
        let features = CString::new("").expect("empty feature string cannot contain NUL");
        let options = if is_elf { options } else { ObjectEmissionOptions::default() };
        let raw = unsafe {
            LLVMCreateTargetMachine(
                llvm_target,
                triple.as_ptr(),
                cpu.as_ptr(),
                features.as_ptr(),
                match options.optimization_level {
                    ObjectOptimizationLevel::None => LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
                    ObjectOptimizationLevel::Default => {
                        LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault
                    }
                },
                if position_independent {
                    LLVMRelocMode::LLVMRelocPIC
                } else {
                    LLVMRelocMode::LLVMRelocStatic
                },
                LLVMCodeModel::LLVMCodeModelDefault,
            )
        };
        if raw.is_null() {
            return Err(BackendEmitError("LLVM failed to create target machine".to_owned()));
        }
        super::shim::apply(
            raw,
            super::shim::TargetMachineOptions {
                emit_stack_size_section: options.emit_stack_size_section,
                function_sections: options.function_sections,
            },
        );
        Ok(Self { raw })
    }
}

impl Drop for TargetMachine {
    fn drop(&mut self) {
        unsafe { LLVMDisposeTargetMachine(self.raw) }
    }
}

fn format_llvm_error(message: *mut std::ffi::c_char, fallback: &str) -> String {
    if message.is_null() {
        return fallback.to_owned();
    }
    let text = unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned();
    unsafe { LLVMDisposeMessage(message) };
    format!("{fallback}: {text}")
}
