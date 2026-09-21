#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryStatus {
    Active,
    Deprecated,
}

impl RegistryStatus {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}
