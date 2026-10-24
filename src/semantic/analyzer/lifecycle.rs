use super::super::errors::SemanticError;
use super::Analyzer;
use crate::ast::Program;

impl Analyzer {
    pub(crate) fn imported_member_name<'a>(&self, name: &'a str) -> Option<&'a str> {
        let (namespace, member) = name.split_once("::")?;
        self.imported_namespaces.contains_key(namespace).then_some(member)
    }

    pub(crate) fn normalize_imported_type_name(
        &self,
        type_name: &crate::ast::TypeName,
    ) -> crate::ast::TypeName {
        crate::ast::TypeName {
            name: self.imported_member_name(&type_name.name).unwrap_or(&type_name.name).to_owned(),
            arguments: type_name
                .arguments
                .iter()
                .map(|argument| self.normalize_imported_type_name(argument))
                .collect(),
            reference_role: type_name.reference_role.clone(),
            span: type_name.span,
        }
    }

    pub(crate) fn normalize_imported_type_key(&self, name: &str) -> String {
        let (prefix, name) = ["abs ", "ins ", "erg ", "dat "]
            .iter()
            .find_map(|prefix| name.strip_prefix(prefix).map(|rest| (*prefix, rest)))
            .unwrap_or(("", name));
        let normalized = if let Some(open) = name.find('[') {
            let base = self.normalize_imported_type_key(&name[..open]);
            let inner = &name[open + 1..name.len().saturating_sub(1)];
            let mut arguments = Vec::new();
            let mut start = 0usize;
            let mut depth = 0usize;
            for (index, character) in inner.char_indices() {
                match character {
                    '[' => depth += 1,
                    ']' => depth = depth.saturating_sub(1),
                    ',' if depth == 0 => {
                        arguments
                            .push(self.normalize_imported_type_key(inner[start..index].trim()));
                        start = index + 1;
                    }
                    _ => {}
                }
            }
            if !inner.trim().is_empty() {
                arguments.push(self.normalize_imported_type_key(inner[start..].trim()));
            }
            format!("{base}[{}]", arguments.join(","))
        } else {
            name.split_once("::")
                .filter(|(namespace, _)| self.imported_namespaces.contains_key(*namespace))
                .map(|(_, member)| member.to_owned())
                .or_else(|| name.rsplit_once("::").map(|(_, member)| member.to_owned()))
                .unwrap_or_else(|| name.to_owned())
        };
        format!("{prefix}{normalized}")
    }

    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn analyze(
        mut self,
        program: &Program,
    ) -> Result<super::super::model::SemanticModel, SemanticError> {
        self.register_import_namespaces(program);
        self.register_enums(program)?;
        self.register_roles(program)?;
        self.register_constants(program)?;
        self.validate_pack_declarations(program)?;
        self.validate_serialization_declarations(program)?;
        self.register_structs(program)?;
        self.validate_role_declarations()?;
        self.validate_performances(program)?;
        self.validate_recursive_types()?;
        self.register_declarations(program)?;
        self.collect_dynamic_roles(program);
        self.validate_method_declarations(program)?;
        self.analyze_verbs(program)?;
        Ok(self.finalize_model())
    }

    fn register_import_namespaces(&mut self, program: &Program) {
        for declaration in &program.declarations {
            match declaration {
                crate::ast::TopLevelDecl::Import(import) if !import.is_open => {
                    let binding = import
                        .alias
                        .clone()
                        .or_else(|| import.path.rsplit("::").next().map(str::to_owned))
                        .unwrap_or_else(|| import.path.clone());
                    self.imported_namespaces.insert(binding, import.path.clone());
                }
                crate::ast::TopLevelDecl::ExternalVerb(verb) if verb.module_import => {
                    if let Some((namespace, _)) = verb.name.rsplit_once("::") {
                        self.imported_namespaces
                            .entry(namespace.to_owned())
                            .or_insert_with(|| namespace.to_owned());
                    }
                }
                _ => {}
            }
        }
    }

    fn finalize_model(mut self) -> super::super::model::SemanticModel {
        self.model.generic_instances = self.generic_instances.into_instances();
        self.model.reachable_performances = self.reachable_performances.into_iter().collect();
        self.model.reachable_performances.sort_by(|left, right| {
            (&left.target_type, &left.role_name, &left.method_name).cmp(&(
                &right.target_type,
                &right.role_name,
                &right.method_name,
            ))
        });
        self.model.drop_types = self.drop_types.into_iter().collect();
        self.model.drop_types.sort();
        self.model.binding_type_names = self.binding_type_names.clone();
        self.model.arena_provenance = self.binding_arena_provenance.clone();
        self.model
    }
}
