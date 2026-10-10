use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, Program, TopLevelDecl, TypeName, VerbDecl};

mod expression;
mod support;

use expression::SharedExpressionProvider;
use support::unsupported;

use super::{
    BackendBlockId, BackendContractError, BackendExternal, BackendLayout, BackendLayoutProvider,
    BackendModule, BackendSignature, BackendSymbolBindings, BackendValueType, SymbolIdentity,
    lower_parameter, lower_return, lower_verb_signature, lower_verb_with_expressions,
    lower_verb_with_expressions_as,
};

struct ScalarLayouts;

impl BackendLayoutProvider for ScalarLayouts {
    fn layout_for(&self, _type_name: &TypeName) -> Option<BackendLayout> {
        None
    }
}

#[doc(hidden)]
pub fn lower_scalar_program(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
) -> Result<BackendModule, BackendContractError> {
    lower_program_to_backend_module(namespace, program, entry_name, None)
}

pub(crate) fn lower_program_to_backend_module(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BackendModule, BackendContractError> {
    lower_program_to_backend_module_with_layouts(
        namespace,
        program,
        entry_name,
        bindings,
        &ScalarLayouts,
    )
}

pub(crate) fn lower_program_to_backend_module_with_layouts(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendModule, BackendContractError> {
    let (verbs, externals) = scalar_declarations(program)?;
    let signatures = verbs
        .iter()
        .map(|verb| lower_verb_signature(namespace, verb, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let external_signatures = externals
        .iter()
        .map(|external| lower_external_signature(external, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let return_types = shared_return_types(&verbs, &externals, &signatures, &external_signatures);
    let external_symbols = scalar_external_symbols(namespace, &externals, bindings)?;
    let mut functions = Vec::with_capacity(verbs.len());
    for verb in verbs {
        functions.push(lower_shared_verb(
            namespace,
            verb,
            entry_name,
            layouts,
            &return_types,
            &external_symbols,
        )?);
    }
    let module = BackendModule {
        externals: externals
            .iter()
            .zip(external_signatures)
            .map(|(external, signature)| scalar_external(external, signature, &external_symbols))
            .collect::<Result<_, _>>()?,
        functions,
    };
    module.validate()?;
    Ok(module)
}

fn scalar_declarations(
    program: &Program,
) -> Result<(Vec<&VerbDecl>, Vec<&ExternalVerbDecl>), BackendContractError> {
    let mut verbs = Vec::new();
    let mut externals = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => verbs.push(verb),
            TopLevelDecl::ExternalVerb(external) => externals.push(external),
            _ => {}
        }
    }
    Ok((verbs, externals))
}

fn scalar_external_symbols(
    namespace: &str,
    externals: &[&ExternalVerbDecl],
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BTreeMap<String, SymbolIdentity>, BackendContractError> {
    externals
        .iter()
        .map(|external| {
            let source = external.native_symbol.as_deref().unwrap_or(&external.name);
            let symbol = bindings
                .map(|bindings| bindings.external_symbol(namespace, source))
                .unwrap_or(source);
            Ok((
                external.name.clone(),
                SymbolIdentity::external_raw(symbol)
                    .map_err(|error| unsupported(external.span, error.to_string()))?,
            ))
        })
        .collect()
}

fn scalar_external(
    external: &ExternalVerbDecl,
    signature: BackendSignature,
    symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<BackendExternal, BackendContractError> {
    let symbol = symbols.get(&external.name).cloned().ok_or_else(|| {
        unsupported(external.span, format!("external symbol `{}` is unavailable", external.name))
    })?;
    Ok(BackendExternal {
        symbol,
        parameters: signature.parameters,
        return_value: signature.return_value,
    })
}

fn lower_external_signature(
    external: &ExternalVerbDecl,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendSignature, BackendContractError> {
    let symbol = SymbolIdentity::external_raw(&external.name)
        .map_err(|error| unsupported(external.span, error.to_string()))?;
    let parameters = external
        .params
        .iter()
        .map(|parameter| lower_parameter(parameter, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let return_value = lower_return(external.return_type.as_ref(), layouts)?;
    Ok(BackendSignature { symbol, parameters, return_value })
}

fn shared_return_types(
    verbs: &[&VerbDecl],
    externals: &[&ExternalVerbDecl],
    signatures: &[BackendSignature],
    external_signatures: &[BackendSignature],
) -> BTreeMap<String, Option<BackendValueType>> {
    verbs
        .iter()
        .zip(signatures)
        .map(|(verb, signature)| {
            (
                verb.name.clone(),
                (signature.return_value.ty != BackendValueType::Unit)
                    .then_some(signature.return_value.ty),
            )
        })
        .chain(externals.iter().zip(external_signatures).map(|(external, signature)| {
            (
                external.name.clone(),
                (signature.return_value.ty != BackendValueType::Unit)
                    .then_some(signature.return_value.ty),
            )
        }))
        .collect()
}

fn lower_shared_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
    layouts: &impl BackendLayoutProvider,
    return_types: &BTreeMap<String, Option<BackendValueType>>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<super::BackendFunction, BackendContractError> {
    let mut provider =
        SharedExpressionProvider::new(namespace, verb, return_types, external_symbols);
    match entry_name.filter(|name| *name == verb.name) {
        Some(_) => lower_verb_with_expressions_as(
            SymbolIdentity::entry(&verb.name)
                .map_err(|error| unsupported(verb.span, error.to_string()))?,
            verb,
            layouts,
            &mut provider,
            BackendBlockId(0),
        ),
        None => {
            lower_verb_with_expressions(namespace, verb, layouts, &mut provider, BackendBlockId(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BackendLayout, BackendLayoutProvider, lower_program_to_backend_module_with_layouts,
    };
    use crate::ast::TypeName;
    use crate::codegen::{BackendPassingMode, BackendReturnMode, BackendValueType};
    use crate::lexer::scan;
    use crate::parser::parse;

    struct RegionLayouts;

    impl BackendLayoutProvider for RegionLayouts {
        fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
            (type_name.name == "Region").then_some(BackendLayout { size: 32, alignment: 8 })
        }
    }

    #[test]
    fn lowers_region_signature_and_identity_return_through_shared_pipeline() {
        let source = "verb inspect(abs region: Region[u32]) -> Region[u32] { return region; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("Region fixture should parse");
        let module = lower_program_to_backend_module_with_layouts(
            "actus_root",
            &program,
            None,
            None,
            &RegionLayouts,
        )
        .expect("Region signature should lower through the shared pipeline");
        let function = &module.functions[0];
        assert!(matches!(function.parameters[0].ty, BackendValueType::Region { .. }));
        assert!(matches!(function.parameters[0].passing, BackendPassingMode::Borrowed { .. }));
        assert!(matches!(function.return_value.passing, BackendReturnMode::ReturnSlot { .. }));
    }
}
