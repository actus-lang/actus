use super::super::super::{BackendValueId, BackendValueType};

#[derive(Clone)]
pub(crate) struct LoopValues {
    pub(crate) header: BackendValueId,
    pub(crate) body: BackendValueId,
    pub(crate) increment: BackendValueId,
    pub(crate) one: BackendValueId,
    pub(crate) next: BackendValueId,
    pub(crate) condition: BackendValueId,
    pub(crate) header_carried: Vec<BackendValueId>,
    pub(crate) body_carried: Vec<BackendValueId>,
    pub(crate) increment_carried: Vec<BackendValueId>,
    pub(crate) exit_carried: Vec<BackendValueId>,
}

pub(crate) struct CarriedBinding {
    pub(crate) name: String,
    pub(crate) value: BackendValueId,
    pub(crate) ty: BackendValueType,
    pub(crate) nominal_type: Option<crate::ast::TypeName>,
}
