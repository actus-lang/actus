use std::collections::{HashMap, HashSet};

use crate::ast::{EnumDef, EnumPayload, Program, StructDef, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn register_enums(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Enum(definition) = declaration else { continue };
            if self.enum_types.insert(definition.name.clone(), definition.clone()).is_some() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateEnumName { name: definition.name.clone() },
                    span: definition.span,
                });
            }
        }
        let definitions = self.enum_types.values().cloned().collect::<Vec<_>>();
        for definition in definitions {
            self.validate_enum_definition(&definition)?;
        }
        Ok(())
    }

    fn validate_enum_definition(&mut self, definition: &EnumDef) -> Result<(), SemanticError> {
        self.with_generic_scope(&definition.generic_parameters, |analyzer| {
            for variant in &definition.variants {
                let types = match &variant.payload {
                    EnumPayload::Unit => Vec::new(),
                    EnumPayload::Tuple(types) => types.iter().collect(),
                    EnumPayload::Struct(fields) => fields.iter().map(|field| &field.ty).collect(),
                };
                for type_name in types {
                    analyzer.validate_type_reference(type_name)?;
                }
            }
            Ok(())
        })
    }

    pub(super) fn validate_recursive_types(&self) -> Result<(), SemanticError> {
        let graph = self.type_dependency_graph();
        let mut visited = HashSet::new();
        for name in graph.keys() {
            if let Some(cycle) = find_cycle(name, &graph, &mut Vec::new(), &mut visited) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::RecursiveType { name: cycle },
                    span: self.type_span(name),
                });
            }
        }
        Ok(())
    }

    fn type_dependency_graph(&self) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();
        for (name, definition) in &self.struct_types {
            graph.insert(name.clone(), struct_dependencies(definition, self));
        }
        for (name, definition) in &self.enum_types {
            graph.insert(name.clone(), enum_dependencies(definition, self));
        }
        graph
    }

    fn type_span(&self, name: &str) -> SourceSpan {
        self.struct_types
            .get(name)
            .map(|definition| definition.span)
            .or_else(|| self.enum_types.get(name).map(|definition| definition.span))
            .unwrap_or(SourceSpan::new(0, 0))
    }
}

fn struct_dependencies(definition: &StructDef, analyzer: &Analyzer) -> Vec<String> {
    definition
        .fields
        .iter()
        .filter_map(|field| named_dependency(&field.ty.name, analyzer))
        .collect()
}

fn enum_dependencies(definition: &EnumDef, analyzer: &Analyzer) -> Vec<String> {
    definition
        .variants
        .iter()
        .flat_map(|variant| match &variant.payload {
            EnumPayload::Unit => Vec::new(),
            EnumPayload::Tuple(types) => types
                .iter()
                .filter_map(|type_name| named_dependency(&type_name.name, analyzer))
                .collect(),
            EnumPayload::Struct(fields) => fields
                .iter()
                .filter_map(|field| named_dependency(&field.ty.name, analyzer))
                .collect(),
        })
        .collect()
}

fn named_dependency(name: &str, analyzer: &Analyzer) -> Option<String> {
    (analyzer.struct_types.contains_key(name) || analyzer.enum_types.contains_key(name))
        .then(|| name.to_owned())
}

fn find_cycle(
    name: &str,
    graph: &HashMap<String, Vec<String>>,
    path: &mut Vec<String>,
    visited: &mut HashSet<String>,
) -> Option<String> {
    if let Some(position) = path.iter().position(|candidate| candidate == name) {
        return path.get(position).cloned();
    }
    if !visited.insert(name.to_owned()) {
        return None;
    }
    path.push(name.to_owned());
    for dependency in graph.get(name).into_iter().flatten() {
        if let Some(cycle) = find_cycle(dependency, graph, path, visited) {
            return Some(cycle);
        }
    }
    path.pop();
    None
}
