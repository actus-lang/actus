#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RegistryToken {
    _private: (),
}

impl RegistryToken {
    pub(in crate::modules::aggregation::registry) fn new() -> Self {
        Self { _private: () }
    }
}
