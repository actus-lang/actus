use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SymbolKind {
    Verb,
    External,
    Struct,
    Pack,
    Enum,
    Role,
    Performance,
    Vtable,
    Layout,
    Generic,
    Data,
    Cleanup,
}

impl SymbolKind {
    fn tag(self) -> &'static str {
        match self {
            Self::Verb => "verb",
            Self::External => "extern",
            Self::Struct => "struct",
            Self::Pack => "pack",
            Self::Enum => "enum",
            Self::Role => "role",
            Self::Performance => "performance",
            Self::Vtable => "vtable",
            Self::Layout => "layout",
            Self::Generic => "generic",
            Self::Data => "data",
            Self::Cleanup => "cleanup",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SymbolIdentity {
    encoded: String,
}

impl SymbolIdentity {
    pub fn new(namespace_prefix: &str, kind: SymbolKind, name: &str) -> Result<Self, SymbolError> {
        Self::specialized(namespace_prefix, kind, name, &[])
    }

    pub fn specialized(
        namespace_prefix: &str,
        kind: SymbolKind,
        name: &str,
        specialization: &[&str],
    ) -> Result<Self, SymbolError> {
        if namespace_prefix.is_empty() {
            return Err(SymbolError::EmptyNamespace);
        }
        if name.is_empty() {
            return Err(SymbolError::EmptyName);
        }
        let suffix = specialization.iter().map(|part| encode(part)).collect::<Vec<_>>().join("_");
        let encoded = match suffix.is_empty() {
            true => format!("{namespace_prefix}__{}_{}", kind.tag(), encode(name)),
            false => format!("{namespace_prefix}__{}_{}__{suffix}", kind.tag(), encode(name)),
        };
        Ok(Self { encoded })
    }

    pub fn as_str(&self) -> &str {
        &self.encoded
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SymbolError {
    EmptyNamespace,
    EmptyName,
    Duplicate(String),
}

impl Display for SymbolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyNamespace => formatter.write_str("symbol namespace cannot be empty"),
            Self::EmptyName => formatter.write_str("symbol name cannot be empty"),
            Self::Duplicate(symbol) => write!(formatter, "duplicate symbol identity `{symbol}`"),
        }
    }
}

impl std::error::Error for SymbolError {}

#[derive(Default)]
pub struct SymbolRegistry {
    symbols: BTreeSet<SymbolIdentity>,
}

impl SymbolRegistry {
    pub fn register(&mut self, symbol: SymbolIdentity) -> Result<(), SymbolError> {
        if self.symbols.insert(symbol.clone()) {
            Ok(())
        } else {
            Err(SymbolError::Duplicate(symbol.encoded))
        }
    }
}

fn encode(value: &str) -> String {
    value.bytes().fold(String::new(), |mut result, byte| {
        if byte.is_ascii_alphanumeric() {
            result.push(byte as char);
        } else {
            result.push('_');
            result.push_str(&format!("{byte:02x}"));
        }
        result
    })
}
