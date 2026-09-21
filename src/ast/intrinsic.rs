#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntrinsicKind {
    Allocate,
    Append,
    Drop,
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
            Self::Drop => IntrinsicSpec { name: "drop", parameters: &["binding"] },
        }
    }
}

pub fn lookup_intrinsic(name: &str) -> Option<IntrinsicKind> {
    match name {
        "allocate" => Some(IntrinsicKind::Allocate),
        "append" => Some(IntrinsicKind::Append),
        "drop" => Some(IntrinsicKind::Drop),
        _ => None,
    }
}

pub fn lookup_call_intrinsic(name: &str) -> Option<IntrinsicKind> {
    match lookup_intrinsic(name) {
        Some(IntrinsicKind::Allocate) | Some(IntrinsicKind::Append) => lookup_intrinsic(name),
        Some(IntrinsicKind::Drop) | None => None,
    }
}
