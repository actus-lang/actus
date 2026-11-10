pub(in crate::semantic) mod arguments;
mod effects;
mod ownership;
mod parsing;
mod paths;
pub(in crate::semantic) mod shapes;
mod signatures;
mod validation;

use crate::semantic::{analyzer, errors, model, state};
pub(crate) use parsing::parse_type_name_key;
pub(super) use signatures::VerbSignature;
