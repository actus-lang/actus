use super::support::unsupported;
use super::{
    BackendContractError, BackendExternal, BackendFunction, BackendInstruction, BackendParameter,
    BackendPassingMode, BackendReturn, BackendReturnMode, BackendValueType, SymbolIdentity,
    SymbolKind,
};

pub(crate) fn append_print_externals(
    externals: &mut Vec<BackendExternal>,
    required_symbols: &std::collections::BTreeSet<String>,
) -> Result<(), BackendContractError> {
    for (symbol, ty) in [
        (crate::runtime::PRINT_INT_SYMBOL, BackendValueType::Integer { signed: true, width: 32 }),
        (crate::runtime::PRINT_STRING_SYMBOL, BackendValueType::Pointer),
        (crate::runtime::WRITE_STRING_STDOUT_SYMBOL, BackendValueType::Pointer),
        (crate::runtime::PRINT_BUFFER_STDOUT_SYMBOL, BackendValueType::Pointer),
    ] {
        if !required_symbols.contains(symbol) {
            continue;
        }
        if externals.iter().any(|external| external.symbol.as_str() == symbol) {
            continue;
        }
        externals.push(BackendExternal {
            symbol: SymbolIdentity::external_raw(symbol).map_err(|error| {
                unsupported(crate::lexer::SourceSpan::new(0, 0), error.to_string())
            })?,
            parameters: vec![BackendParameter {
                name: "value".to_owned(),
                role: crate::ast::Role::Erg,
                ty,
                passing: super::BackendPassingMode::Direct,
            }],
            return_value: super::BackendReturn {
                role: None,
                ty: BackendValueType::Integer { signed: true, width: 32 },
                passing: super::BackendReturnMode::Direct,
            },
            runtime_pointer_return: false,
        });
    }
    Ok(())
}

pub(crate) fn referenced_runtime_symbols(
    functions: &[BackendFunction],
) -> std::collections::BTreeSet<String> {
    let mut symbols = functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction {
            BackendInstruction::Call { callee, .. } if callee.kind() == SymbolKind::External => {
                Some(callee.as_str().to_owned())
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    if symbols.contains(crate::runtime::REGION_OPEN_SYMBOL)
        || functions.iter().any(|function| {
            function.parameters.iter().any(|parameter| {
                parameter.ty == BackendValueType::Pointer
                    && matches!(parameter.passing, BackendPassingMode::OwnershipTransfer { .. })
            })
        })
    {
        symbols.insert(crate::runtime::BUFFER_DROP_SYMBOL.to_owned());
    }
    symbols
}
pub(crate) fn append_performance_externals(
    externals: &mut Vec<BackendExternal>,
    performance_externals: &[BackendExternal],
) -> Result<(), BackendContractError> {
    let mut known = externals
        .iter()
        .map(|external| external.symbol.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for external in performance_externals {
        if let Some(existing) =
            externals.iter().find(|candidate| candidate.symbol == external.symbol)
        {
            if existing.parameters != external.parameters
                || existing.return_value != external.return_value
                || existing.runtime_pointer_return != external.runtime_pointer_return
            {
                return Err(BackendContractError::InvalidExternalSymbol);
            }
            continue;
        }
        if known.insert(external.symbol.clone()) {
            externals.push(external.clone());
        }
    }
    Ok(())
}
pub(crate) fn append_buffer_allocate_external(
    externals: &mut Vec<BackendExternal>,
    required_symbols: &std::collections::BTreeSet<String>,
) -> Result<(), BackendContractError> {
    if !required_symbols.contains(crate::runtime::BUFFER_ALLOCATE_SYMBOL) {
        return Ok(());
    }
    if externals
        .iter()
        .any(|external| external.symbol.as_str() == crate::runtime::BUFFER_ALLOCATE_SYMBOL)
    {
        return Ok(());
    }
    externals.push(buffer_allocate_external()?);
    Ok(())
}

pub(crate) fn append_buffer_drop_external(
    externals: &mut Vec<BackendExternal>,
    required_symbols: &std::collections::BTreeSet<String>,
) -> Result<(), BackendContractError> {
    let symbol = crate::runtime::BUFFER_DROP_SYMBOL;
    if !required_symbols.contains(symbol)
        || externals.iter().any(|external| external.symbol.as_str() == symbol)
    {
        return Ok(());
    }
    externals.push(BackendExternal {
        symbol: SymbolIdentity::external_raw(symbol)
            .map_err(|error| unsupported(crate::lexer::SourceSpan::new(0, 0), error.to_string()))?,
        parameters: vec![BackendParameter {
            name: "handle".to_owned(),
            role: crate::ast::Role::Erg,
            ty: BackendValueType::Pointer,
            passing: BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        runtime_pointer_return: false,
    });
    Ok(())
}

fn buffer_allocate_external() -> Result<BackendExternal, BackendContractError> {
    Ok(BackendExternal {
        symbol: SymbolIdentity::external_raw(crate::runtime::BUFFER_ALLOCATE_SYMBOL)
            .map_err(|error| unsupported(crate::lexer::SourceSpan::new(0, 0), error.to_string()))?,
        parameters: vec![BackendParameter {
            name: "length".to_owned(),
            role: crate::ast::Role::Erg,
            ty: BackendValueType::Integer { signed: false, width: 64 },
            passing: super::BackendPassingMode::Direct,
        }],
        return_value: super::BackendReturn {
            role: Some(crate::ast::Role::Erg),
            ty: BackendValueType::Pointer,
            passing: super::BackendReturnMode::Direct,
        },
        runtime_pointer_return: false,
    })
}
