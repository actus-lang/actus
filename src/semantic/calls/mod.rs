pub(in crate::semantic) mod arguments;
mod dispatch;
mod effects;
mod ownership;
mod paths;
pub(in crate::semantic) mod shapes;
mod signatures;
mod validation;

pub(crate) use crate::ast::parse_type_name_key;
use crate::semantic::{analyzer, errors, model, state};
pub(super) use signatures::VerbSignature;
