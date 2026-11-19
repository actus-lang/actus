#[cfg_attr(not(feature = "cranelift"), allow(dead_code))]
mod definitions;
mod identity;
mod instance_expressions;
mod specializations;
mod verb_body;
mod verbs;

#[cfg(feature = "cranelift")]
pub(crate) use definitions::{
    canonical_type_name, collect_concrete_type_instances, specialized_enums, specialized_structs,
};
pub(crate) use identity::instances_of_verb;
pub(crate) use verbs::{expand_generic_instances, specialize_program, specialized_generic_name};
