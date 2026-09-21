#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForeignAbi {
    C,
}

impl ForeignAbi {
    pub const fn name(self) -> &'static str {
        match self {
            Self::C => "C",
        }
    }
}
