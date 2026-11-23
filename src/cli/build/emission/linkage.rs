//! One native object per program (ADR-0113).
//!
//! The lowered modules of every unit are linked into one backend module
//! before emission, so the program becomes a single relocatable object.
//! References between units resolve inside that object, and the only global
//! symbol the object defines is the program entry: every other function has
//! local linkage. External declarations stay undefined references, satisfied
//! by the runtime, a provider, or the host.

use std::collections::{BTreeMap, BTreeSet};

use crate::codegen::{
    BackendEmitError, BackendExternal, BackendModule, BackendParameter, BackendPassingMode,
    BackendValueType,
};

use super::shared::LinkedProgram;

pub(super) fn link_program(
    modules: Vec<BackendModule>,
    entry: &str,
) -> Result<LinkedProgram, BackendEmitError> {
    let mut externals = BTreeMap::<String, BackendExternal>::new();
    let mut functions = Vec::new();
    for module in modules {
        for external in module.externals {
            let symbol = external.symbol.as_str().to_owned();
            let merged = match externals.remove(&symbol) {
                Some(known) => merge_external(known, external)?,
                None => external,
            };
            externals.insert(symbol, merged);
        }
        functions.extend(module.functions);
    }
    let defined = functions
        .iter()
        .map(|function| function.symbol.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let module = BackendModule {
        externals: externals
            .into_values()
            .filter(|external| !defined.contains(external.symbol.as_str()))
            .collect(),
        functions,
        runtime_checks: crate::codegen::RuntimeCheckReporting::Trap,
    };
    let local_symbols = defined.into_iter().filter(|symbol| symbol != entry).collect();
    Ok(LinkedProgram { module, local_symbols })
}

/// Two declarations of one external symbol, from different units, must
/// describe one native call: the same machine parameter and return classes.
/// Parameter names and ownership roles are source-level. A parameter that one
/// declaration passes as a pointer value and the other as a reference to its
/// storage is the same machine pointer; the merged declaration keeps the
/// direct pointer, which carries no alignment or aliasing assumption.
fn merge_external(
    known: BackendExternal,
    other: BackendExternal,
) -> Result<BackendExternal, BackendEmitError> {
    let same_return = known.return_value.ty == other.return_value.ty
        && known.return_value.passing == other.return_value.passing
        && known.runtime_pointer_return == other.runtime_pointer_return;
    let same_parameters = known.parameters.len() == other.parameters.len()
        && known
            .parameters
            .iter()
            .zip(&other.parameters)
            .all(|(left, right)| native_class(left) == native_class(right));
    if !same_return || !same_parameters {
        return Err(BackendEmitError(format!(
            "external `{}` is declared with two different native signatures",
            known.symbol.as_str()
        )));
    }
    let mut merged = known;
    for (parameter, other) in merged.parameters.iter_mut().zip(other.parameters) {
        if matches!(other.passing, BackendPassingMode::Direct) {
            *parameter = other;
        }
    }
    Ok(merged)
}

#[derive(PartialEq)]
enum NativeClass<'a> {
    Scalar(BackendValueType),
    Aggregate(&'a crate::codegen::BackendDirectAggregateAbi),
    Pointer,
}

fn native_class(parameter: &BackendParameter) -> NativeClass<'_> {
    match &parameter.passing {
        BackendPassingMode::Direct if parameter.ty == BackendValueType::Pointer => {
            NativeClass::Pointer
        }
        BackendPassingMode::Direct => NativeClass::Scalar(parameter.ty),
        BackendPassingMode::DirectAggregate { abi } => NativeClass::Aggregate(abi),
        BackendPassingMode::Indirect { .. }
        | BackendPassingMode::Borrowed { .. }
        | BackendPassingMode::ExclusiveLoan { .. }
        | BackendPassingMode::OwnershipTransfer { .. }
        | BackendPassingMode::RuntimeBridge { .. }
        | BackendPassingMode::ExclusiveScalarLoan { .. } => NativeClass::Pointer,
    }
}

#[cfg(test)]
mod tests {
    use super::merge_external;
    use crate::ast::Role;
    use crate::codegen::{
        BackendExternal, BackendLayout, BackendParameter, BackendPassingMode, BackendReturn,
        BackendReturnMode, BackendValueType, SymbolIdentity,
    };

    fn external(name: &str, role: Role, passing: BackendPassingMode) -> BackendExternal {
        let int = BackendValueType::Integer { signed: true, width: 64 };
        BackendExternal {
            symbol: SymbolIdentity::external_raw("actus_buffer_byte_at").expect("symbol"),
            parameters: vec![
                BackendParameter {
                    name: name.to_owned(),
                    role,
                    ty: BackendValueType::Pointer,
                    passing,
                },
                BackendParameter {
                    name: "index".to_owned(),
                    role: Role::Erg,
                    ty: int,
                    passing: BackendPassingMode::Direct,
                },
            ],
            return_value: BackendReturn { role: None, ty: int, passing: BackendReturnMode::Direct },
            runtime_pointer_return: false,
        }
    }

    /// One runtime symbol declared by two units: names and roles differ, and
    /// one passes the handle by reference to its storage. Both are one machine
    /// pointer, so they merge into the direct declaration.
    #[test]
    fn equivalent_external_declarations_merge_to_the_direct_pointer() {
        let borrowed =
            BackendPassingMode::Borrowed { layout: BackendLayout { size: 8, alignment: 8 } };
        let merged = merge_external(
            external("storage", Role::Abs, borrowed),
            external("handle", Role::Erg, BackendPassingMode::Direct),
        )
        .expect("equivalent declarations merge");
        assert_eq!(merged.parameters[0].passing, BackendPassingMode::Direct);
    }

    /// A pointer and an integer are different native calls.
    #[test]
    fn different_native_signatures_do_not_merge() {
        let mut integer = external("handle", Role::Erg, BackendPassingMode::Direct);
        integer.parameters[0].ty = BackendValueType::Integer { signed: true, width: 64 };
        let error =
            merge_external(external("handle", Role::Erg, BackendPassingMode::Direct), integer)
                .expect_err("different declarations conflict");
        assert!(error.to_string().contains("two different native signatures"), "{error}");
    }
}
