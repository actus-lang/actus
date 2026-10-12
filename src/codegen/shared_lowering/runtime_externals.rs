use crate::codegen::{
    BackendContractError, BackendExternal, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendValueType, SymbolIdentity,
};

use super::support::unsupported;

pub(super) fn append_buffer_runtime_externals(
    externals: &mut Vec<BackendExternal>,
) -> Result<(), BackendContractError> {
    append_buffer_append(externals)?;
    append_buffer_length(externals)?;
    append_buffer_byte_at(externals)?;
    append_buffer_byte_set(externals)?;
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
