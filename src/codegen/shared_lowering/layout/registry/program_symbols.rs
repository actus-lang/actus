use super::*;

/// Drop performance symbols declared by one program, keyed by canonical type.
pub(super) fn collect_drop_symbols(
    program: &Program,
    namespace: &str,
) -> BTreeMap<String, SymbolIdentity> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Perform(perform) if perform.role_name == "Drop" => {
                let lookup_key = perform.target.canonical_lookup_key();
                let symbol_target = perform.target.stable_symbol_name();
                let symbol_namespace =
                    perform.module_import_namespace.as_deref().unwrap_or(namespace);
                let symbol = SymbolIdentity::specialized(
                    symbol_namespace,
                    SymbolKind::Performance,
                    "drop",
                    &["Drop", &symbol_target],
                )
                .ok()?;
                Some((lookup_key, symbol))
            }
            _ => None,
        })
        .collect()
}

pub(super) fn collect_imported_namespaces(program: &Program) -> BTreeMap<String, String> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Import(import) = declaration else { return None };
            (!import.is_open).then(|| {
                let binding = import
                    .alias
                    .clone()
                    .or_else(|| import.path.rsplit("::").next().map(str::to_owned))
                    .unwrap_or_else(|| import.path.clone());
                (binding, import.path.clone())
            })
        })
        .collect()
}
