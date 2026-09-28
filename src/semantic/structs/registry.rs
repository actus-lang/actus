use std::collections::HashSet;

use crate::ast::{Program, StructDef, TopLevelDecl, TypeName};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(crate) fn register_structs(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Struct(definition) = declaration else { continue };
            if self.struct_types.insert(definition.name.clone(), definition.clone()).is_some() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateStructName { name: definition.name.clone() },
                    span: definition.span,
                });
            }
        }
        let definitions = self.struct_types.values().cloned().collect::<Vec<_>>();
        for definition in definitions {
            self.validate_struct_definition(&definition)?;
        }
        Ok(())
    }

    fn validate_struct_definition(&mut self, definition: &StructDef) -> Result<(), SemanticError> {
        self.with_generic_scope(&definition.generic_parameters, |analyzer| {
            let mut field_names = HashSet::new();
            for field in &definition.fields {
                if !field_names.insert(field.name.clone()) {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::DuplicateStructField {
                            struct_name: definition.name.clone(),
                            field: field.name.clone(),
                        },
                        span: field.span,
                    });
                }
                analyzer.validate_type_reference(&field.ty)?;
            }
            Ok(())
        })
    }

    pub(crate) fn record_struct_binding(
        &mut self,
        name: &str,
        type_name: &TypeName,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if !self.struct_types.contains_key(&type_name.name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_struct_types.insert(index, type_name.name.clone());
        if !type_name.arguments.is_empty() {
            self.binding_struct_type_applications.insert(index, type_name.clone());
        }
        Ok(())
    }

    pub(crate) fn record_initializer_struct_type(
        &mut self,
        name: &str,
        initializer: &crate::ast::Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(type_name) = self.resolved_type_name(initializer) else { return Ok(()) };
        if !self.struct_types.contains_key(&type_name.name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_struct_types.insert(index, type_name.name.clone());
        if !type_name.arguments.is_empty() {
            self.binding_struct_type_applications.insert(index, type_name);
        }
        Ok(())
    }

    pub(crate) fn record_initializer_pack_type(
        &mut self,
        name: &str,
        initializer: &crate::ast::Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(type_name) = self.resolved_type_name(initializer) else { return Ok(()) };
        if !self.pack_types.contains_key(&type_name.name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_type_names.insert(index, type_name);
        Ok(())
    }
}
