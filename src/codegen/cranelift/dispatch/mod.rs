mod dynamic;
mod vtable;

pub(super) use dynamic::lower_dynamic_call;
pub(super) use vtable::{
    VtableDataIds, declare_vtable_values, define_vtables, vtable_symbol_for_native_in_namespace,
};
