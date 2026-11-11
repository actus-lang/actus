use std::collections::HashMap;

use crate::ast::TopLevelDecl;

use super::super::collision::{ExternalContract, external_contract};
use super::super::types::ModuleError;
use super::super::unit::ModuleUnit;
use super::super::validation::{export_identity, target_scopes_are_disjoint, target_selector};

pub(super) fn validate_symbol_collisions(
    local_declarations: &[TopLevelDecl],
    units: &[ModuleUnit],
) -> Result<(), ModuleError> {
    let mut symbols = HashMap::new();
    record_symbols("<root>", local_declarations, &mut symbols)?;
    for unit in units {
        record_symbols(
            unit.identity().module_path(),
            &unit.implementation().declarations,
            &mut symbols,
        )?;
    }
    Ok(())
}

fn record_symbols(
    module: &str,
    declarations: &[TopLevelDecl],
    symbols: &mut HashMap<(String, String), Vec<SymbolRecord>>,
) -> Result<(), ModuleError> {
    for declaration in declarations {
        let Some((kind, name)) = export_identity(declaration) else { continue };
        let key = (kind.to_owned(), name.clone());
        let record = SymbolRecord {
            module: module.to_owned(),
            external_contract: external_contract(declaration),
            declaration: declaration.clone(),
        };
        let records = symbols.entry(key).or_default();
        for first in records.iter() {
            if first.module == module
                || target_scopes_are_disjoint(
                    target_selector(&first.declaration),
                    target_selector(&record.declaration),
                )
            {
                continue;
            }
            if first.declaration == *declaration {
                continue;
            }
            if first.external_contract.is_some()
                && first.external_contract == record.external_contract
            {
                continue;
            }
            return Err(ModuleError::SymbolCollision {
                symbol: if record.external_contract.is_some() {
                    format!("native symbol `{name}`")
                } else {
                    format!("{kind} `{name}`")
                },
                first_module: first.module.clone(),
                second_module: module.to_owned(),
            });
        }
        records.push(record);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SymbolRecord {
    module: String,
    external_contract: Option<ExternalContract>,
    declaration: TopLevelDecl,
}
