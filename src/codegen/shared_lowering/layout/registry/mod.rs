use std::collections::BTreeMap;

use crate::ast::{
    BuiltinType, EnumDef, EnumPayload, PackDecl, Program, Role, StructDef, StructFieldRole,
    TopLevelDecl, TypeName, primitive_type,
};

use super::super::super::BackendFieldLayout;
use super::super::super::symbols::{SymbolIdentity, SymbolKind};
use super::super::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupProvider, BackendCleanupResolvedDescriptor,
    BackendCleanupResult, BackendContractError, BackendLayout, BackendLayoutProvider,
};

mod cleanup;
mod options;
mod program_layouts;
mod provider;

use options::{is_niche_pointer_option, niche_option_variant, niche_pointer_variant};

pub(crate) use program_layouts::ProgramLayouts;
