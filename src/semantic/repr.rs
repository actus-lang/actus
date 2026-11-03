use crate::ast::{
    EnumDef, EnumPayload, PrimitiveType, StructDef, StructFieldRole, TypeName, primitive_type,
};

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn validate_repr_declarations(&self) -> Result<(), SemanticError> {
        for definition in self.struct_types.values() {
            if definition.is_volatile()
                || definition.repr().is_some_and(|repr| {
                    repr.is_c()
                        || repr.is_packed()
                        || repr.is_transparent()
                        || repr.alignment().is_some()
                })
            {
                self.validate_repr_struct(definition)?;
            }
        }
        for definition in self.enum_types.values() {
            if definition.repr().is_some_and(|repr| {
                repr.is_c()
                    || repr.is_packed()
                    || repr.is_transparent()
                    || repr.alignment().is_some()
            }) {
                self.validate_repr_enum(definition)?;
            }
        }
        Ok(())
    }

    fn validate_repr_struct(&self, definition: &StructDef) -> Result<(), SemanticError> {
        let repr = definition.repr().unwrap_or(crate::ast::Repr::C);
        if definition.is_volatile() {
            if !definition.repr().is_some_and(|declared| declared.is_c()) {
                return Err(repr_error_at(
                    &definition.name,
                    "volatile values require an explicit repr(C) contract",
                    definition.span,
                ));
            }
            if !definition.generic_parameters.is_empty() {
                return Err(repr_error_at(
                    &definition.name,
                    "volatile values cannot be generic",
                    definition.span,
                ));
            }
            self.validate_volatile_struct_fields(definition)?;
        }
        self.validate_repr_struct_contract(definition, repr)?;
        if !definition.generic_parameters.is_empty() {
            return Err(repr_error_at(
                &definition.name,
                "generic structs cannot declare a concrete native representation",
                definition.span,
            ));
        }
        self.validate_repr_struct_fields(definition)
    }

    fn validate_volatile_struct_fields(&self, definition: &StructDef) -> Result<(), SemanticError> {
        for field in &definition.fields {
            if !matches!(field.role, StructFieldRole::Value) {
                return Err(repr_error_at(
                    &definition.name,
                    format!("volatile field `{}` must use the value role", field.name),
                    field.span,
                ));
            }
            let supported = primitive_type(&field.ty.name).is_some_and(|primitive| {
                matches!(
                    primitive,
                    PrimitiveType::Integer { width, .. } if width.is_multiple_of(8)
                )
            }) || (matches!(field.ty.name.as_str(), "Int" | "Bool")
                && field.ty.arguments.is_empty());
            if !supported {
                return Err(repr_error_at(
                    &definition.name,
                    format!(
                        "volatile field `{}` must use a fixed-width integer or Bool type",
                        field.name
                    ),
                    field.ty.span,
                ));
            }
        }
        Ok(())
    }

    fn validate_repr_struct_contract(
        &self,
        definition: &StructDef,
        repr: crate::ast::Repr,
    ) -> Result<(), SemanticError> {
        if repr.is_transparent() {
            if repr.is_c() || repr.is_packed() || repr.alignment().is_some() {
                return Err(repr_error_at(
                    &definition.name,
                    "transparent representation cannot be combined with C, packed, or explicit alignment",
                    definition.span,
                ));
            }
            if definition.fields.len() != 1 {
                return Err(repr_error_at(
                    &definition.name,
                    "transparent representation requires exactly one field",
                    definition.span,
                ));
            }
        }
        if let Some(alignment) = repr.alignment() {
            if !alignment.is_power_of_two() {
                return Err(repr_error_at(
                    &definition.name,
                    "explicit alignment must be a positive power of two",
                    definition.span,
                ));
            }
            if repr.is_packed() {
                return Err(repr_error_at(
                    &definition.name,
                    "explicit alignment cannot be combined with packed representation",
                    definition.span,
                ));
            }
        }
        Ok(())
    }

    fn validate_repr_struct_fields(&self, definition: &StructDef) -> Result<(), SemanticError> {
        for field in &definition.fields {
            if !matches!(field.role, StructFieldRole::Value) {
                return Err(repr_error_at(
                    &definition.name,
                    format!(
                        "field `{}` uses ownership role `{}`; represented fields must be value fields",
                        field.name,
                        field_role_name(&field.role)
                    ),
                    field.span,
                ));
            }
            self.validate_repr_field_type(
                &definition.name,
                &field.name,
                &field.ty,
                &mut Vec::new(),
            )?;
        }
        Ok(())
    }

    fn validate_repr_enum(&self, definition: &EnumDef) -> Result<(), SemanticError> {
        if definition.repr().is_some_and(|repr| {
            repr.is_packed() || repr.is_transparent() || repr.alignment().is_some()
        }) {
            return Err(repr_error_at(
                &definition.name,
                "packed, transparent, and explicit alignment representations are supported only for structs",
                definition.span,
            ));
        }
        if !definition.generic_parameters.is_empty() {
            return Err(repr_error_at(
                &definition.name,
                "generic enums cannot declare a concrete C ABI representation",
                definition.span,
            ));
        }
        for variant in &definition.variants {
            self.validate_repr_enum_payload(definition, variant)?;
        }
        Ok(())
    }

    fn validate_repr_enum_payload(
        &self,
        definition: &EnumDef,
        variant: &crate::ast::EnumVariant,
    ) -> Result<(), SemanticError> {
        let fields: Vec<(String, &TypeName, crate::lexer::SourceSpan)> = match &variant.payload {
            EnumPayload::Unit => return Ok(()),
            EnumPayload::Tuple(types) => types
                .iter()
                .enumerate()
                .map(|(index, ty)| (index.to_string(), ty, ty.span))
                .collect(),
            EnumPayload::Struct(fields) => {
                fields.iter().map(|field| (field.name.clone(), &field.ty, field.span)).collect()
            }
        };
        for (field, type_name, span) in fields {
            self.validate_repr_field_type(&definition.name, &field, type_name, &mut Vec::new())
                .map_err(|error| match error.kind {
                    SemanticErrorKind::InvalidReprC { .. } => repr_error_at(
                        &definition.name,
                        format!(
                            "variant `{}` field `{field}` has an invalid payload type",
                            variant.name
                        ),
                        span,
                    ),
                    _ => error,
                })?;
        }
        Ok(())
    }

    fn validate_repr_field_type(
        &self,
        declaration: &str,
        field: &str,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Result<(), SemanticError> {
        if type_name.reference_role.is_some() {
            return Err(repr_error_at(
                declaration,
                format!("field `{field}` has an ownership-qualified type reference"),
                type_name.span,
            ));
        }
        if let Some(primitive) = primitive_type(&type_name.name) {
            let supported = match primitive {
                PrimitiveType::Integer { width, .. } => width.is_multiple_of(8),
                PrimitiveType::Float { .. } => true,
                PrimitiveType::Void => false,
            };
            if supported {
                return Ok(());
            }
        }
        if matches!(type_name.name.as_str(), "Int" | "Bool") && type_name.arguments.is_empty() {
            return Ok(());
        }
        if type_name.name == "Array" {
            return self.validate_repr_array(declaration, field, type_name, visiting);
        }
        if let Some(structure) = self.struct_types.get(&type_name.name) {
            return self.validate_repr_struct_field(
                declaration,
                field,
                type_name,
                structure,
                visiting,
            );
        }
        if let Some(enum_definition) = self.enum_types.get(&type_name.name) {
            return self.validate_repr_enum_field(declaration, field, type_name, enum_definition);
        }
        Err(repr_error_at(
            declaration,
            format!("field `{field}` uses unsupported C ABI type `{}`", type_name.canonical_key()),
            type_name.span,
        ))
    }

    fn validate_repr_array(
        &self,
        declaration: &str,
        field: &str,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Result<(), SemanticError> {
        let [element, capacity] = type_name.arguments.as_slice() else {
            return Err(repr_error_at(
                declaration,
                format!("field `{field}` has malformed `Array` type"),
                type_name.span,
            ));
        };
        if !capacity.arguments.is_empty()
            || capacity.reference_role.is_some()
            || capacity.name.parse::<u32>().ok().is_none_or(|value| value == 0)
        {
            return Err(repr_error_at(
                declaration,
                format!("field `{field}` must use a positive concrete array capacity"),
                capacity.span,
            ));
        }
        self.validate_repr_field_type(declaration, field, element, visiting)
    }

    fn validate_repr_struct_field(
        &self,
        declaration: &str,
        field: &str,
        type_name: &TypeName,
        structure: &StructDef,
        visiting: &mut Vec<String>,
    ) -> Result<(), SemanticError> {
        if !structure.repr().is_some_and(|repr| {
            repr.is_c() || repr.is_packed() || repr.is_transparent() || repr.alignment().is_some()
        }) || !type_name.arguments.is_empty()
        {
            return Err(repr_error_at(
                declaration,
                format!(
                    "field `{field}` uses `{}` without a concrete representation contract",
                    type_name.canonical_key()
                ),
                type_name.span,
            ));
        }
        if visiting.iter().any(|name| name == &type_name.name) {
            return Err(repr_error_at(
                declaration,
                format!("field `{field}` creates a recursive C layout"),
                type_name.span,
            ));
        }
        visiting.push(type_name.name.clone());
        for nested in &structure.fields {
            self.validate_repr_field_type(&structure.name, &nested.name, &nested.ty, visiting)?;
        }
        visiting.pop();
        Ok(())
    }

    fn validate_repr_enum_field(
        &self,
        declaration: &str,
        field: &str,
        type_name: &TypeName,
        enumeration: &EnumDef,
    ) -> Result<(), SemanticError> {
        if !enumeration.repr().is_some_and(|repr| repr.is_c()) || !type_name.arguments.is_empty() {
            return Err(repr_error_at(
                declaration,
                format!(
                    "field `{field}` uses `{}` without a concrete repr(C) contract",
                    type_name.canonical_key()
                ),
                type_name.span,
            ));
        }
        for variant in &enumeration.variants {
            self.validate_repr_enum_payload(enumeration, variant)?;
        }
        Ok(())
    }
}

fn repr_error_at(
    declaration: &str,
    reason: impl Into<String>,
    span: crate::lexer::SourceSpan,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidReprC {
            declaration: declaration.to_owned(),
            reason: reason.into(),
        },
        span,
    }
}

fn field_role_name(role: &StructFieldRole) -> &'static str {
    match role {
        StructFieldRole::Value => "value",
        StructFieldRole::Erg => "erg",
        StructFieldRole::Abs => "abs",
        StructFieldRole::Ins => "ins",
    }
}
