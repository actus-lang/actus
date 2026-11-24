use std::fmt;
use std::hash::{Hash, Hasher};

use crate::lexer::SourceSpan;

use super::interning::DeclarationId;

use crate::modules::aggregation::registry::RegistryToken;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Role {
    Erg,
    Abs,
    Dat,
    Ins,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclarationPath {
    package: String,
    module_path: String,
    name: String,
}

impl DeclarationPath {
    pub(crate) fn new(
        _token: RegistryToken,
        package: impl Into<String>,
        module_path: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        let package = package.into();
        let module_path = module_path.into();
        assert!(!package.is_empty(), "declaration identity requires a package");
        assert!(!module_path.is_empty(), "declaration identity requires a module path");
        Self { package, module_path, name: name.into() }
    }

    pub(in crate::ast) fn decoded(
        package: impl Into<String>,
        module_path: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        let package = package.into();
        let module_path = module_path.into();
        assert!(!package.is_empty(), "decoded declaration identity requires a package");
        assert!(!module_path.is_empty(), "decoded declaration identity requires a module path");
        Self { package, module_path, name: name.into() }
    }

    pub fn package(&self) -> &str {
        &self.package
    }

    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeclarationIdentity {
    Declaration(DeclarationPath),
    Builtin { name: String },
    GenericParameter { owner: String, name: String },
    Const { value: String },
    Unknown { spelling: String },
}

impl DeclarationIdentity {
    pub fn constant(value: impl Into<String>) -> Self {
        Self::Const { value: normalize_const_value(&value.into()) }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Declaration(path) => {
                format!("{}::{}::{}", path.package(), path.module_path(), path.name())
            }
            Self::Builtin { name } => format!("builtin::{name}"),
            Self::GenericParameter { owner, name } => format!("{owner}::{name}"),
            Self::Const { value } => format!("const::{value}"),
            Self::Unknown { spelling } => format!("unknown::{spelling}"),
        }
    }
}

#[derive(Clone)]
pub struct TypeName {
    pub name: String,
    pub arguments: Vec<TypeName>,
    pub reference_role: Option<Role>,
    pub span: SourceSpan,
    pub resolved: Option<DeclarationId>,
}

impl fmt::Debug for TypeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TypeName")
            .field("name", &self.name)
            .field("arguments", &self.arguments)
            .field("reference_role", &self.reference_role)
            .field("span", &self.span)
            .finish()
    }
}

/// The type as written in source: `Result[Int, IoError]`.
impl fmt::Display for TypeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.name)?;
        if let Some((first, rest)) = self.arguments.split_first() {
            write!(formatter, "[{first}")?;
            for argument in rest {
                write!(formatter, ", {argument}")?;
            }
            formatter.write_str("]")?;
        }
        Ok(())
    }
}

impl TypeName {
    pub fn unresolved(
        name: impl Into<String>,
        arguments: Vec<TypeName>,
        reference_role: Option<Role>,
        span: SourceSpan,
    ) -> Self {
        Self { name: name.into(), arguments, reference_role, span, resolved: None }
    }

    pub fn resolved(
        identity: DeclarationIdentity,
        spelling: impl Into<String>,
        arguments: Vec<TypeName>,
        reference_role: Option<Role>,
        span: SourceSpan,
    ) -> Self {
        Self::resolved_id(
            DeclarationId::intern(identity),
            spelling,
            arguments,
            reference_role,
            span,
        )
    }

    pub fn resolved_id(
        identity: DeclarationId,
        spelling: impl Into<String>,
        arguments: Vec<TypeName>,
        reference_role: Option<Role>,
        span: SourceSpan,
    ) -> Self {
        Self { name: spelling.into(), arguments, reference_role, span, resolved: Some(identity) }
    }

    pub fn builtin(
        name: impl Into<String>,
        arguments: Vec<TypeName>,
        reference_role: Option<Role>,
        span: SourceSpan,
    ) -> Self {
        let name = name.into();
        Self::resolved(
            DeclarationIdentity::Builtin { name: name.clone() },
            name,
            arguments,
            reference_role,
            span,
        )
    }

    pub fn constant(
        value: impl Into<String>,
        spelling: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::resolved(DeclarationIdentity::constant(value), spelling, Vec::new(), None, span)
    }
}

impl DeclarationIdentity {
    pub fn generic_parameter(owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self::GenericParameter { owner: owner.into(), name: name.into() }
    }
}

fn normalize_const_value(value: &str) -> String {
    let compact = value.replace('_', "");
    let split = compact.find(|character: char| character.is_ascii_alphabetic());
    let (digits, suffix) = split.map_or((compact.as_str(), ""), |index| compact.split_at(index));
    let normalized_digits = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .and_then(|hex| u128::from_str_radix(hex, 16).ok())
        .map(|number| number.to_string())
        .or_else(|| digits.parse::<u128>().ok().map(|number| number.to_string()))
        .unwrap_or_else(|| digits.to_owned());
    format!("{normalized_digits}{suffix}")
}

impl PartialEq for TypeName {
    fn eq(&self, other: &Self) -> bool {
        self.identity_key() == other.identity_key()
            && self.arguments == other.arguments
            && self.reference_role == other.reference_role
    }
}

impl Eq for TypeName {}

impl Hash for TypeName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity_key().hash(state);
        self.arguments.hash(state);
        self.reference_role.hash(state);
    }
}

impl TypeName {
    fn identity_key(&self) -> TypeNameKey<'_> {
        match &self.resolved {
            Some(identity) => TypeNameKey::Resolved(identity),
            None => TypeNameKey::Unresolved(&self.name),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum TypeNameKey<'a> {
    Resolved(&'a DeclarationId),
    Unresolved(&'a str),
}

/// Structural identity for a nominal type application.
///
/// Source spans, ownership roles, facade paths, and call-site metadata are
/// intentionally excluded from this value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeIdentity {
    identity: Option<DeclarationId>,
    name: String,
    arguments: Vec<TypeIdentity>,
}

impl TypeIdentity {
    pub fn from_type_name(type_name: &TypeName) -> Self {
        Self {
            identity: type_name.resolved,
            name: type_name.name.clone(),
            arguments: type_name.arguments.iter().map(Self::from_type_name).collect(),
        }
    }

    pub fn from_verb_application(name: &str, arguments: &[TypeName]) -> Self {
        Self {
            identity: None,
            name: name.to_owned(),
            arguments: arguments.iter().map(Self::from_type_name).collect(),
        }
    }

    pub fn key(&self) -> String {
        let mut key = String::new();
        self.write_key(&mut key);
        key
    }

    fn write_key(&self, output: &mut String) {
        write_canonical_key(self.identity, &self.name, &self.arguments, output, Self::write_key);
    }

    pub fn verb_key(&self) -> String {
        if self.arguments.is_empty() {
            return self.identity.map_or_else(|| self.name.clone(), DeclarationId::key);
        }
        let name = self.identity.map_or_else(|| self.name.clone(), DeclarationId::key);
        format!(
            "{}[{}]",
            name,
            self.arguments.iter().map(Self::verb_key).collect::<Vec<_>>().join(",")
        )
    }
}

/// Writes `identity[argument,...]` without intermediate allocations.
fn write_canonical_key<T>(
    identity: Option<DeclarationId>,
    name: &str,
    arguments: &[T],
    output: &mut String,
    write_argument: fn(&T, &mut String),
) {
    let identity = identity.unwrap_or_else(|| {
        panic!("internal compiler error: unresolved type identity `{name}` reached type key")
    });
    output.push_str(identity.canonical_key());
    if arguments.is_empty() {
        return;
    }
    output.push('[');
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_argument(argument, output);
    }
    output.push(']');
}

impl TypeName {
    pub fn canonical_key(&self) -> String {
        let mut key = String::new();
        self.write_canonical_key(&mut key);
        key
    }

    fn write_canonical_key(&self, output: &mut String) {
        write_canonical_key(
            self.resolved,
            &self.name,
            &self.arguments,
            output,
            Self::write_canonical_key,
        );
    }

    pub fn spelling_key(&self) -> String {
        if self.arguments.is_empty() {
            return self.name.clone();
        }
        format!(
            "{}[{}]",
            self.name,
            self.arguments.iter().map(Self::spelling_key).collect::<Vec<_>>().join(",")
        )
    }

    /// Canonical key for compiler-internal lookup tables and caches.
    pub fn canonical_lookup_key(&self) -> String {
        assert!(
            self.resolved.is_some(),
            "unresolved TypeName `{}` reached canonical lookup",
            self.name
        );
        self.canonical_key()
    }

    /// Stable declaration-based name for ABI and runtime symbol contracts.
    pub fn stable_symbol_name(&self) -> String {
        let identity = self.resolved.unwrap_or_else(|| {
            panic!("unresolved type `{}` reached stable symbol naming", self.name)
        });
        let identity = identity.identity();
        let name = match identity {
            DeclarationIdentity::Declaration(path) => path.name(),
            DeclarationIdentity::Builtin { name }
            | DeclarationIdentity::GenericParameter { name, .. } => name,
            DeclarationIdentity::Const { value } => value,
            DeclarationIdentity::Unknown { spelling } => spelling,
        };
        if self.arguments.is_empty() {
            return name.to_owned();
        }
        format!(
            "{}[{}]",
            name,
            self.arguments.iter().map(Self::stable_symbol_name).collect::<Vec<_>>().join(",")
        )
    }

    pub fn identity(&self) -> TypeIdentity {
        TypeIdentity::from_type_name(self)
    }

    pub fn resolved_identity(&self) -> Option<&'static DeclarationIdentity> {
        self.resolved.map(DeclarationId::identity)
    }
}

#[cfg(test)]
mod tests {
    use super::{Role, TypeIdentity, TypeName};
    use crate::lexer::SourceSpan;

    fn type_name(name: &str, arguments: Vec<TypeName>, span: usize) -> TypeName {
        TypeName::builtin(name, arguments, None, SourceSpan::new(span, span + name.len()))
    }

    #[test]
    fn canonical_key_excludes_source_span_and_reference_role() {
        let mut first = type_name("Result", vec![type_name("Int", Vec::new(), 0)], 10);
        let mut second = type_name("Result", vec![type_name("Int", Vec::new(), 200)], 500);
        first.reference_role = Some(Role::Dat);
        second.reference_role = Some(Role::Abs);

        assert_eq!(first.canonical_key(), "builtin::Result[builtin::Int]");
        assert_eq!(first.canonical_key(), second.canonical_key());
    }

    #[test]
    fn canonical_key_preserves_ordered_generic_arguments() {
        let first = type_name(
            "Pair",
            vec![type_name("Int", Vec::new(), 0), type_name("Bool", Vec::new(), 10)],
            20,
        );
        let second = type_name(
            "Pair",
            vec![type_name("Bool", Vec::new(), 30), type_name("Int", Vec::new(), 40)],
            50,
        );

        assert_eq!(first.canonical_key(), "builtin::Pair[builtin::Int,builtin::Bool]");
        assert_eq!(second.canonical_key(), "builtin::Pair[builtin::Bool,builtin::Int]");
        assert_ne!(first.canonical_key(), second.canonical_key());
    }

    #[test]
    fn identity_is_a_structural_value_object() {
        let type_name = type_name("Result", vec![type_name("Int", Vec::new(), 0)], 10);
        let identity = TypeIdentity::from_type_name(&type_name);

        assert_eq!(identity.verb_key(), "builtin::Result[builtin::Int]");
        assert_eq!(identity, type_name.identity());
    }

    #[test]
    fn canonical_key_for_large_extent_is_bounded() {
        let large_type = type_name(
            "Array",
            vec![
                type_name("u8", Vec::new(), 0),
                TypeName::constant("1048576", "1048576", SourceSpan::new(3, 10)),
            ],
            0,
        );

        assert_eq!(large_type.canonical_key(), "builtin::Array[builtin::u8,const::1048576]");
        let small = type_name(
            "Array",
            vec![
                type_name("u8", Vec::new(), 0),
                TypeName::constant("4", "4", SourceSpan::new(3, 4)),
            ],
            0,
        );
        assert_eq!(large_type.canonical_key().len() - small.canonical_key().len(), 6);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnAccess {
    Owned,
    Abs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReturnType {
    pub access: ReturnAccess,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub kind: GenericParamKind,
    pub bound: Option<TypeName>,
    pub bounds: Vec<TypeName>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenericParamKind {
    Type,
    Const { domain: TypeName },
}
