use crate::ast::{
    EnumDef, EnumPayload, PackDecl, PrimitiveType, StructDef, StructFieldRole, TypeName,
    primitive_type,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

/// Nesting depth at which a type is assumed not to be a copy type, so a
/// self-referential array element cannot loop forever.
const COPY_SEARCH_DEPTH: usize = 8;

/// One field of a struct or one payload of an enum variant under `meta copy`.
struct CopyField<'a> {
    name: String,
    type_name: &'a TypeName,
    span: SourceSpan,
    /// An `abs` or `ins` field is a view, which is never a copy type.
    view: bool,
}

impl Analyzer {
    /// Whether a value of this type may be duplicated by the written
    /// `copy(value: abs x)` (ADR-0124): a scalar, a struct, enum or pack
    /// declared `meta copy`, or an array of copy types. A view never is.
    pub(in crate::semantic) fn type_is_copy(&self, type_name: &TypeName) -> bool {
        self.type_is_copy_within(type_name, 0)
    }

    /// Whether the named type is a scalar or a struct, enum or pack declared
    /// `meta copy`; the head of a generic instance such as `Pair[Int]` names
    /// the declaration. Arrays are copy types only as fields, not as `copy`
    /// operands.
    pub(in crate::semantic) fn named_type_is_copy_operand(&self, name: &str) -> bool {
        let head = name.split('[').next().unwrap_or(name);
        if head == "Int"
            || head == "Bool"
            || primitive_type(head)
                .is_some_and(|primitive| !matches!(primitive, PrimitiveType::Void))
        {
            return true;
        }
        self.struct_types.get(head).is_some_and(StructDef::is_copy)
            || self.enum_types.get(head).is_some_and(EnumDef::is_copy)
            || self.pack_types.get(head).is_some_and(PackDecl::is_copy)
    }

    fn type_is_copy_within(&self, type_name: &TypeName, depth: usize) -> bool {
        if depth > COPY_SEARCH_DEPTH || type_name.reference_role.is_some() {
            return false;
        }
        let type_name = self.normalize_imported_type_name(type_name);
        let name = type_name.name.as_str();
        if name == "Array" {
            return type_name
                .arguments
                .first()
                .is_some_and(|element| self.type_is_copy_within(element, depth + 1));
        }
        self.named_type_is_copy_operand(name)
    }

    /// Rejects `meta copy` on a struct, enum or pack with a drop contract or
    /// with a field or payload that is not a copy type (E1146). Declarations
    /// are checked in source order so the first offender reported is
    /// deterministic.
    pub(in crate::semantic) fn validate_copy_declarations(&self) -> Result<(), SemanticError> {
        for (name, span, fields) in self.copy_declarations() {
            if self.drop_types.contains(name) {
                return Err(copy_error(name, "it has a drop contract".to_owned(), span));
            }
            for field in fields {
                let type_name = super::analyzer::canonical_type_name(field.type_name);
                let reason = if field.view {
                    format!(
                        "field `{}` is a view of `{type_name}`, which is not a copy",
                        field.name
                    )
                } else if !self.type_is_copy(field.type_name) {
                    format!(
                        "field `{}` has type `{type_name}`, which is not a copy type",
                        field.name
                    )
                } else {
                    continue;
                };
                return Err(copy_error(name, reason, field.span));
            }
        }
        Ok(())
    }

    /// Every struct, enum and pack declared `meta copy` with its fields, in
    /// source order.
    fn copy_declarations(&self) -> Vec<(&str, SourceSpan, Vec<CopyField<'_>>)> {
        let mut declarations: Vec<(&str, SourceSpan, Vec<CopyField<'_>>)> = Vec::new();
        for definition in self.struct_types.values().filter(|definition| definition.is_copy()) {
            let fields = definition
                .fields
                .iter()
                .map(|field| CopyField {
                    name: field.name.clone(),
                    type_name: &field.ty,
                    span: field.span,
                    view: matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins),
                })
                .collect();
            declarations.push((definition.name.as_str(), definition.span, fields));
        }
        for definition in self.enum_types.values().filter(|definition| definition.is_copy()) {
            let payloads = enum_copy_fields(definition);
            declarations.push((definition.name.as_str(), definition.span, payloads));
        }
        for definition in self.pack_types.values().filter(|definition| definition.is_copy()) {
            let fields = definition
                .fields
                .iter()
                .map(|field| CopyField {
                    name: field.name.clone(),
                    type_name: &field.ty,
                    span: field.span,
                    view: false,
                })
                .collect();
            declarations.push((definition.name.as_str(), definition.span, fields));
        }
        declarations.sort_by_key(|(name, span, _)| (span.start, *name));
        declarations
    }
}

/// Every payload of every variant of an enum, named `Variant.0` when
/// positional and `Variant.name` when named.
fn enum_copy_fields(definition: &EnumDef) -> Vec<CopyField<'_>> {
    let mut payloads = Vec::new();
    for variant in &definition.variants {
        match &variant.payload {
            EnumPayload::Unit => {}
            EnumPayload::Tuple(types) => {
                payloads.extend(types.iter().enumerate().map(|(index, ty)| CopyField {
                    name: format!("{}.{index}", variant.name),
                    type_name: ty,
                    span: ty.span,
                    view: false,
                }))
            }
            EnumPayload::Struct(fields) => payloads.extend(fields.iter().map(|field| CopyField {
                name: format!("{}.{}", variant.name, field.name),
                type_name: &field.ty,
                span: field.span,
                view: false,
            })),
        }
    }
    payloads
}

fn copy_error(declaration: &str, reason: String, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidCopyType { declaration: declaration.to_owned(), reason },
        span,
    }
}
