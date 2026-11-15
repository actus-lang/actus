use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use super::resolution;

use crate::ast::{DeclarationId, DeclarationIdentity, DeclarationPath, Program, TopLevelDecl};

use super::super::plan::ModuleCompilationPlan;
use super::super::unit::ModuleUnit;
use super::super::validation::{declarations_are_identical_without_spans, export_identity};

type ResolutionCache = Arc<RwLock<BTreeMap<(String, String), Option<DeclarationId>>>>;
type IdentityComparisonCache = Arc<RwLock<BTreeMap<(String, String), bool>>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegisteredDeclaration {
    identity: DeclarationIdentity,
    kind: String,
    source: Arc<Program>,
    declaration_index: usize,
}

impl RegisteredDeclaration {
    pub(crate) fn is_type(&self) -> bool {
        matches!(self.kind.as_str(), "struct" | "enum" | "pack")
    }

    fn declaration(&self) -> &TopLevelDecl {
        self.source.declarations.get(self.declaration_index).expect("registered declaration index")
    }
}

#[derive(Clone, Debug, Default)]
pub struct DeclarationRegistry {
    pub(in crate::modules::aggregation::registry) declarations:
        BTreeMap<String, RegisteredDeclaration>,
    pub(in crate::modules::aggregation::registry) imports:
        BTreeMap<String, Vec<crate::ast::ImportDecl>>,
    pub(in crate::modules::aggregation::registry) index:
        BTreeMap<(String, String), Vec<DeclarationIdentity>>,
    pub(in crate::modules::aggregation::registry) resolution_cache: ResolutionCache,
    pub(in crate::modules::aggregation::registry) identity_comparison_cache:
        IdentityComparisonCache,
}

impl PartialEq for DeclarationRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.declarations == other.declarations
            && self.imports == other.imports
            && self.index == other.index
    }
}

impl Eq for DeclarationRegistry {}

impl DeclarationRegistry {
    /// Resolves parser-only programs that have no manifest or module resolver.
    /// `standalone` is the documented package identity for this tooling path.
    pub(crate) fn resolve_standalone(program: &mut crate::ast::Program) -> Self {
        let mut registry = Self::default();
        registry.register_program(
            super::STANDALONE_PACKAGE_IDENTITY,
            super::ROOT_MODULE_PATH,
            program,
        );
        registry.annotate_program_headers(super::ROOT_MODULE_PATH, program);
        registry
    }

    pub(crate) fn from_plan_with_session(
        plan: &ModuleCompilationPlan,
        session: Option<&crate::session::CompilationSession>,
    ) -> Self {
        let mut registry = Self::default();
        registry.register_program(plan.package_identity(), super::ROOT_MODULE_PATH, plan.caller());
        for unit in plan.units() {
            if let Some(session) = session {
                let key = unit_declaration_key(plan, unit);
                let view = session.unit_declarations(key, || Self::from_unit(unit));
                registry.merge_from(&view);
            } else {
                registry.register_unit(unit);
            }
        }
        registry
    }

    pub(crate) fn from_unit(unit: &ModuleUnit) -> Self {
        let mut registry = Self::default();
        registry.register_unit(unit);
        registry
    }

    pub(crate) fn merge_from(&mut self, other: &Self) {
        for (key, declaration) in &other.declarations {
            self.declarations.entry(key.clone()).or_insert_with(|| declaration.clone());
        }
        for (module, imports) in &other.imports {
            self.imports.entry(module.clone()).or_default().extend(imports.iter().cloned());
        }
        for (key, identities) in &other.index {
            let target = self.index.entry(key.clone()).or_default();
            for identity in identities {
                if !target.contains(identity) {
                    target.push(identity.clone());
                }
            }
        }
    }

    pub(crate) fn resolve(&self, module_path: &str, spelling: &str) -> Option<DeclarationIdentity> {
        self.resolve_id(module_path, spelling).map(|id| id.identity().clone())
    }

    /// Cached `(module, spelling)` resolution to an interned identity.
    pub(crate) fn resolve_id(&self, module_path: &str, spelling: &str) -> Option<DeclarationId> {
        let cache_key = (module_path.to_owned(), spelling.to_owned());
        if let Some(cached) =
            self.resolution_cache.read().expect("resolution cache poisoned").get(&cache_key)
        {
            return *cached;
        }
        let resolved = self.resolve_uncached(module_path, spelling).map(DeclarationId::intern);
        self.resolution_cache
            .write()
            .expect("resolution cache poisoned")
            .insert(cache_key, resolved);
        resolved
    }

    fn resolve_uncached(&self, module_path: &str, spelling: &str) -> Option<DeclarationIdentity> {
        let (prefix, short_name) =
            spelling.rsplit_once("::").map_or((None, spelling), |(p, n)| (Some(p), n));
        let imported_module = self.imported_module(module_path, prefix, short_name);
        let mut qualified = Vec::new();
        if let Some(owner) = imported_module.or(prefix) {
            self.append_indexed_candidates(owner, short_name, &mut qualified);
        } else {
            self.append_indexed_candidates(module_path, short_name, &mut qualified);
            if let Some(imports) = self.imports.get(module_path) {
                for import in imports.iter().filter(|import| {
                    import.is_open || import.selected.iter().any(|name| name == short_name)
                }) {
                    self.append_indexed_candidates(&import.path, short_name, &mut qualified);
                }
            }
        }
        match qualified.as_slice() {
            [single] => Some(single.clone()),
            [first, rest @ ..]
                if rest.iter().all(|candidate| {
                    let left = self.declarations.get(&first.key()).expect("registered identity");
                    let right =
                        self.declarations.get(&candidate.key()).expect("registered identity");
                    left.kind == right.kind && self.identical_declarations(first, candidate)
                }) =>
            {
                Some(first.clone())
            }
            [] if resolution::is_builtin_name(spelling) => {
                Some(DeclarationIdentity::Builtin { name: spelling.to_owned() })
            }
            [] => Some(DeclarationIdentity::Unknown { spelling: spelling.to_owned() }),
            _ => Some(DeclarationIdentity::Unknown { spelling: spelling.to_owned() }),
        }
    }

    fn append_indexed_candidates(
        &self,
        module_path: &str,
        name: &str,
        candidates: &mut Vec<DeclarationIdentity>,
    ) {
        let key = (module_path.to_owned(), name.to_owned());
        if let Some(indexed) = self.index.get(&key) {
            for identity in indexed {
                if !candidates.contains(identity) {
                    candidates.push(identity.clone());
                }
            }
        }
    }

    fn identical_declarations(
        &self,
        first: &DeclarationIdentity,
        second: &DeclarationIdentity,
    ) -> bool {
        let first_key = first.key();
        let second_key = second.key();
        let pair = if first_key <= second_key {
            (first_key.clone(), second_key.clone())
        } else {
            (second_key.clone(), first_key.clone())
        };
        if let Some(result) = self
            .identity_comparison_cache
            .read()
            .expect("identity comparison cache poisoned")
            .get(&pair)
            .copied()
        {
            return result;
        }
        let left = self.declarations.get(&first_key).expect("registered identity");
        let right = self.declarations.get(&second_key).expect("registered identity");
        let result =
            declarations_are_identical_without_spans(left.declaration(), right.declaration());
        self.identity_comparison_cache
            .write()
            .expect("identity comparison cache poisoned")
            .insert(pair, result);
        result
    }

    pub(crate) fn resolves_type(&self, module_path: &str, spelling: &str) -> bool {
        let Some(identity) = self.resolve(module_path, spelling) else { return false };
        if matches!(identity, DeclarationIdentity::Builtin { .. }) {
            return true;
        }
        self.declarations.get(&identity.key()).is_some_and(RegisteredDeclaration::is_type)
    }

    fn imported_module(
        &self,
        module_path: &str,
        prefix: Option<&str>,
        short_name: &str,
    ) -> Option<&str> {
        let prefix = prefix?;
        self.imports.get(module_path)?.iter().find_map(|import| {
            let alias = import.alias.as_deref().or_else(|| import.path.rsplit("::").next());
            (alias == Some(prefix)
                || import.path == prefix
                || (import.is_open
                    && import.path == prefix
                    && import.selected.iter().any(|name| name == short_name)))
            .then_some(import.path.as_str())
        })
    }

    #[cfg(test)]
    pub(crate) fn resolve_type_name(
        &self,
        module_path: &str,
        type_name: &mut crate::ast::TypeName,
    ) {
        self.resolve_scoped_type_name(super::annotation::Scope::module(module_path), type_name);
    }

    /// Resolves one type spelling in `scope`: const literal, generic parameter
    /// of the declaration being annotated, then the module's declarations.
    pub(in crate::modules::aggregation::registry) fn resolve_scoped_type_name(
        &self,
        scope: super::annotation::Scope<'_>,
        type_name: &mut crate::ast::TypeName,
    ) {
        for argument in &mut type_name.arguments {
            self.resolve_scoped_type_name(scope, argument);
        }
        if type_name.resolved.is_some() {
            return;
        }
        type_name.resolved = if is_const_literal(&type_name.name) {
            Some(DeclarationId::intern(DeclarationIdentity::constant(&type_name.name)))
        } else if scope.generic_parameters.iter().any(|parameter| parameter.name == type_name.name)
        {
            Some(DeclarationId::intern(DeclarationIdentity::GenericParameter {
                owner: scope.generic_owner.to_owned(),
                name: type_name.name.clone(),
            }))
        } else {
            self.resolve_id(scope.module_path, &type_name.name)
        };
    }

    /// Canonical key of the generic declaration `name` that owns `parameters`.
    pub(in crate::modules::aggregation::registry) fn generic_owner(
        &self,
        scope: super::annotation::Scope<'_>,
        name: &str,
        parameters: &[crate::ast::GenericParam],
    ) -> String {
        if parameters.is_empty() {
            return String::new();
        }
        let registered = self
            .index
            .get(&(scope.module_path.to_owned(), name.to_owned()))
            .and_then(|identities| identities.first())
            .map(DeclarationIdentity::key);
        registered
            .or_else(|| self.resolve_id(scope.module_path, name).map(DeclarationId::key))
            .unwrap_or_else(|| {
                panic!(
                    "internal compiler error: generic declaration `{name}` is not registered in module `{}`",
                    scope.module_path
                )
            })
    }

    fn register_unit(&mut self, unit: &ModuleUnit) {
        self.register_program(
            unit.identity().package(),
            unit.identity().module_path(),
            unit.implementation(),
        );
    }

    pub(in crate::modules::aggregation::registry) fn register_program(
        &mut self,
        package: &str,
        module_path: &str,
        program: &crate::ast::Program,
    ) {
        self.imports.entry(module_path.to_owned()).or_default().extend(
            program.declarations.iter().filter_map(|declaration| match declaration {
                TopLevelDecl::Import(import) => Some(import.clone()),
                _ => None,
            }),
        );
        let source = Arc::new(program.clone());
        for (declaration_index, declaration) in program.declarations.iter().enumerate() {
            // An imported verb is a binding to its source declaration, which
            // its own module registers; it is not a declaration of the
            // importer (ADR-0108).
            if matches!(declaration, TopLevelDecl::ExternalVerb(verb) if verb.module_import) {
                continue;
            }
            let Some((kind, name)) = export_identity(declaration) else { continue };
            let identity = DeclarationIdentity::Declaration(DeclarationPath::new(
                super::RegistryToken::new(),
                package,
                module_path,
                name,
            ));
            self.insert_registered(identity, kind, source.clone(), declaration_index);
        }
    }

    #[cfg(test)]
    pub(crate) fn register_declaration(
        &mut self,
        package: &str,
        module_path: &str,
        declaration: &TopLevelDecl,
    ) {
        if let Some((kind, name)) = export_identity(declaration).or_else(|| {
            let (kind, name) = match declaration {
                TopLevelDecl::Struct(value) => ("struct", &value.name),
                TopLevelDecl::Enum(value) => ("enum", &value.name),
                TopLevelDecl::Pack(value) => ("pack", &value.name),
                TopLevelDecl::Role(value) => ("role", &value.name),
                TopLevelDecl::Verb(value) => ("verb", &value.name),
                TopLevelDecl::ExternalVerb(value) => ("external", &value.name),
                TopLevelDecl::Constant(value) => ("constant", &value.name),
                TopLevelDecl::Serialize(value) => ("serialize", &value.name),
                TopLevelDecl::Perform(_)
                | TopLevelDecl::Import(_)
                | TopLevelDecl::OpenSibling(_) => return None,
            };
            Some((kind, name.clone()))
        }) {
            let identity = DeclarationIdentity::Declaration(DeclarationPath::new(
                super::RegistryToken::new(),
                package,
                module_path,
                name,
            ));
            let source = Arc::new(Program {
                file_metadata: Vec::new(),
                declarations: vec![declaration.clone()],
            });
            self.insert_registered(identity, kind, source, 0);
        }
    }

    fn insert_registered(
        &mut self,
        identity: DeclarationIdentity,
        kind: &str,
        source: Arc<Program>,
        declaration_index: usize,
    ) {
        let key = identity.key();
        if self.declarations.contains_key(&key) {
            return;
        }
        let DeclarationIdentity::Declaration(path) = &identity else { return };
        self.index
            .entry((path.module_path().to_owned(), path.name().to_owned()))
            .or_default()
            .push(identity.clone());
        self.declarations.insert(
            key,
            RegisteredDeclaration { identity, kind: kind.to_owned(), source, declaration_index },
        );
    }
}

fn unit_declaration_key(plan: &ModuleCompilationPlan, unit: &ModuleUnit) -> String {
    let closure = plan
        .units()
        .iter()
        .map(|dependency| dependency.identity().source_key())
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{}::{}@{}#{}",
        unit.identity().package(),
        unit.identity().module_path(),
        unit.identity().source_key(),
        closure
    )
}

fn is_const_literal(value: &str) -> bool {
    value.chars().next().is_some_and(|character| character.is_ascii_digit())
}
