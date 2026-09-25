#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntrinsicKind {
    Append,
    Print,
    Drop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntrinsicSpec {
    pub name: &'static str,
    pub parameters: &'static [&'static str],
    pub status: RegistryStatus,
}

impl IntrinsicKind {
    pub const fn spec(self) -> IntrinsicSpec {
        match self {
            Self::Append => IntrinsicSpec {
                name: "append",
                parameters: &["handle", "byte"],
                status: RegistryStatus::Active,
            },
            Self::Print => IntrinsicSpec {
                name: "print",
                parameters: &["value"],
                status: RegistryStatus::Active,
            },
            Self::Drop => IntrinsicSpec {
                name: "drop",
                parameters: &["binding"],
                status: RegistryStatus::Active,
            },
        }
    }
}

pub fn lookup_intrinsic(name: &str) -> Option<IntrinsicKind> {
    match name {
        "append" => Some(IntrinsicKind::Append),
        "print" => Some(IntrinsicKind::Print),
        "drop" => Some(IntrinsicKind::Drop),
        _ => None,
    }
}

pub fn lookup_call_intrinsic(name: &str) -> Option<IntrinsicKind> {
    match lookup_intrinsic(name) {
        Some(IntrinsicKind::Append) | Some(IntrinsicKind::Print) => lookup_intrinsic(name),
        Some(IntrinsicKind::Drop) | None => None,
    }
}
use super::RegistryStatus;
