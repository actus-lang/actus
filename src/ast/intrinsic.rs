#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntrinsicKind {
    Allocate,
    Append,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntrinsicSpec {
    pub name: &'static str,
    pub parameters: &'static [&'static str],
}

impl IntrinsicKind {
    pub const fn spec(self) -> IntrinsicSpec {
        match self {
            Self::Allocate => IntrinsicSpec { name: "allocate", parameters: &["length"] },
            Self::Append => IntrinsicSpec { name: "append", parameters: &["handle", "byte"] },
        }
    }
}

pub fn lookup_intrinsic(name: &str) -> Option<IntrinsicKind> {
    match name {
        "allocate" => Some(IntrinsicKind::Allocate),
        "append" => Some(IntrinsicKind::Append),
        _ => None,
    }
}
