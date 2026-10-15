use crate::codegen::{BackendContractError, SymbolIdentity, SymbolKind};
use crate::lexer::SourceSpan;

use super::super::SharedExpressionProvider;

pub(super) fn callee_for(
    provider: &SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
) -> Result<SymbolIdentity, BackendContractError> {
    provider
        .external_symbols
        .get(callee)
        .cloned()
        .or_else(|| {
            provider
                .bindings
                .map(|bindings| bindings.external_symbol(provider.namespace, callee))
                .filter(|symbol| *symbol != callee)
                .and_then(|symbol| SymbolIdentity::external_raw(symbol).ok())
        })
        .or_else(|| {
            provider
                .return_types
                .contains_key(callee)
                .then(|| SymbolIdentity::new(provider.namespace, SymbolKind::Verb, callee))
                .and_then(Result::ok)
        })
        .map_or_else(|| SymbolIdentity::new(provider.namespace, SymbolKind::Verb, callee), Ok)
        .map_err(|error| super::super::super::support::unsupported(span, error.to_string()))
}
