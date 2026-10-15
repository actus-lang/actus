use crate::codegen::{
    BackendContractError, BackendExternal, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendValueType, SymbolIdentity,
};

use super::support::unsupported;

pub(super) fn append_buffer_runtime_externals(
    externals: &mut Vec<BackendExternal>,
    required_symbols: &std::collections::BTreeSet<String>,
) -> Result<(), BackendContractError> {
    let start = externals.len();
    append_buffer_append(externals)?;
    append_buffer_length(externals)?;
    append_buffer_byte_at(externals)?;
    append_buffer_byte_set(externals)?;
    append_buffer_crc32(externals)?;
    append_buffer_crc32_matches(externals)?;
    append_buffer_validate_fixed_frame(externals)?;
    let candidates = externals
        .drain(start..)
        .filter(|external| required_symbols.contains(external.symbol.as_str()))
        .collect::<Vec<_>>();
    externals.extend(candidates);
    Ok(())
}

fn append_buffer_append(externals: &mut Vec<BackendExternal>) -> Result<(), BackendContractError> {
    append_runtime_external(
        externals,
        crate::runtime::BUFFER_APPEND_SYMBOL,
        vec![buffer_param("handle"), byte_param("byte")],
        byte_type(),
    )
}

fn append_buffer_length(externals: &mut Vec<BackendExternal>) -> Result<(), BackendContractError> {
    append_runtime_external(
        externals,
        crate::runtime::BUFFER_LENGTH_SYMBOL,
        vec![buffer_param("buffer")],
        int_type(),
    )
}

fn append_buffer_byte_at(externals: &mut Vec<BackendExternal>) -> Result<(), BackendContractError> {
    append_runtime_external(
        externals,
        crate::runtime::BUFFER_BYTE_AT_SYMBOL,
        vec![buffer_param("handle"), index_param()],
        int_type(),
    )
}

fn append_buffer_byte_set(
    externals: &mut Vec<BackendExternal>,
) -> Result<(), BackendContractError> {
    append_runtime_external(
        externals,
        crate::runtime::BUFFER_BYTE_SET_SYMBOL,
        vec![buffer_param("handle"), index_param(), byte_param("byte")],
        int_type(),
    )
}

fn append_buffer_crc32(externals: &mut Vec<BackendExternal>) -> Result<(), BackendContractError> {
    append_runtime_external(
        externals,
        crate::runtime::BUFFER_CRC32_SYMBOL,
        vec![buffer_param("buffer"), index64_param("start"), index64_param("end")],
        int64_type(),
    )
}

fn append_buffer_crc32_matches(
    externals: &mut Vec<BackendExternal>,
) -> Result<(), BackendContractError> {
    append_runtime_external(
        externals,
        crate::runtime::BUFFER_CRC32_MATCHES_SYMBOL,
        vec![
            buffer_param("buffer"),
            index64_param("start"),
            index64_param("end"),
            index64_param("expected"),
        ],
        int64_type(),
    )
}

fn append_buffer_validate_fixed_frame(
    externals: &mut Vec<BackendExternal>,
) -> Result<(), BackendContractError> {
    append_runtime_external(
        externals,
        crate::runtime::BUFFER_VALIDATE_FIXED_FRAME_SYMBOL,
        vec![
            buffer_param("buffer"),
            index64_param("little"),
            index64_param("version_offset"),
            index64_param("expected_version"),
            index64_param("payload_offset"),
            index64_param("payload_length"),
            index64_param("checksum_start"),
            index64_param("checksum_end"),
            index64_param("checksum_offset"),
        ],
        int64_type(),
    )
}

fn buffer_param(name: &str) -> BackendParameter {
    BackendParameter {
        name: name.to_owned(),
        role: crate::ast::Role::Erg,
        ty: BackendValueType::Pointer,
        passing: BackendPassingMode::Direct,
    }
}

fn index_param() -> BackendParameter {
    BackendParameter {
        name: "index".to_owned(),
        role: crate::ast::Role::Erg,
        ty: BackendValueType::Integer { signed: true, width: 32 },
        passing: BackendPassingMode::Direct,
    }
}

fn index64_param(name: &str) -> BackendParameter {
    BackendParameter {
        name: name.to_owned(),
        role: crate::ast::Role::Erg,
        ty: int64_type(),
        passing: BackendPassingMode::Direct,
    }
}

fn byte_param(name: &str) -> BackendParameter {
    BackendParameter {
        name: name.to_owned(),
        role: crate::ast::Role::Erg,
        ty: BackendValueType::Integer { signed: false, width: 8 },
        passing: BackendPassingMode::Direct,
    }
}

fn byte_type() -> BackendValueType {
    BackendValueType::Integer { signed: false, width: 8 }
}

fn int_type() -> BackendValueType {
    BackendValueType::Integer { signed: true, width: 32 }
}

fn int64_type() -> BackendValueType {
    BackendValueType::Integer { signed: true, width: 64 }
}

fn append_runtime_external(
    externals: &mut Vec<BackendExternal>,
    symbol: &str,
    parameters: Vec<BackendParameter>,
    return_type: BackendValueType,
) -> Result<(), BackendContractError> {
    if externals.iter().any(|external| external.symbol.as_str() == symbol) {
        return Ok(());
    }
    externals.push(BackendExternal {
        symbol: SymbolIdentity::external_raw(symbol)
            .map_err(|error| unsupported(crate::lexer::SourceSpan::new(0, 0), error.to_string()))?,
        parameters,
        return_value: BackendReturn {
            role: None,
            ty: return_type,
            passing: BackendReturnMode::Direct,
        },
        runtime_pointer_return: false,
    });
    Ok(())
}
