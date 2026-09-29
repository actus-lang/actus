use super::{Formatter, meta_name, role_name};

impl Formatter<'_> {
    pub(super) fn role_definition(&mut self, role: &crate::ast::RoleDecl) {
        self.documentation(role.doc.as_deref());
        if role.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("role ");
        self.output.push_str(&role.name);
        self.output.push_str(" {");
        for method in &role.methods {
            self.output.push('\n');
            self.emit_comments_before(method.span.start);
            self.documentation(method.doc.as_deref());
            self.output.push_str(" verb ");
            self.output.push_str(&method.name);
            self.parameters(&method.params);
            self.return_type(&method.return_type);
            self.output.push(';');
        }
        self.output.push_str(" }");
    }

    pub(super) fn perform_definition(&mut self, perform: &crate::ast::PerformDecl) {
        self.documentation(perform.doc.as_deref());
        if perform.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("perform ");
        self.output.push_str(&perform.role_name);
        self.output.push_str(" for ");
        self.type_name(&perform.target);
        self.output.push_str(" {");
        for method in &perform.methods {
            self.output.push(' ');
            self.verb(method);
        }
        self.output.push_str(" }");
    }

    pub(super) fn verb(&mut self, verb: &crate::ast::VerbDecl) {
        self.documentation(verb.doc.as_deref());
        for metadata in &verb.metadata {
            self.output.push_str("meta ");
            self.output.push_str(&meta_name(metadata));
            self.output.push('\n');
        }
        if verb.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("verb ");
        self.output.push_str(&verb.name);
        self.generic_parameters(&verb.generic_parameters);
        self.parameters(&verb.params);
        self.return_type(&verb.return_type);
        self.output.push(' ');
        self.block(&verb.body);
    }

    pub(super) fn external_verb(&mut self, verb: &crate::ast::ExternalVerbDecl) {
        self.documentation(verb.doc.as_deref());
        if verb.is_open {
            self.output.push_str("open ");
        }
        if verb.unsafe_boundary {
            self.output.push_str("unsafe ");
        }
        self.output.push_str("extern \"");
        self.output.push_str(verb.abi.name());
        self.output.push_str("\" verb ");
        self.output.push_str(&verb.name);
        self.generic_parameters(&verb.generic_parameters);
        self.parameters(&verb.params);
        self.return_type(&verb.return_type);
        self.output.push(';');
    }

    fn parameters(&mut self, parameters: &[crate::ast::Param]) {
        if self.should_wrap_parameters(parameters) {
            self.multiline_parameters(parameters);
            return;
        }
        self.output.push('(');
        for (index, parameter) in parameters.iter().enumerate() {
            if index > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(role_name(&parameter.role));
            self.output.push(' ');
            self.output.push_str(&parameter.name);
            self.output.push_str(": ");
            self.type_name(&parameter.ty);
        }
        self.output.push(')');
    }

    fn multiline_parameters(&mut self, parameters: &[crate::ast::Param]) {
        self.output.push_str("(\n");
        self.indent += 1;
        for (index, parameter) in parameters.iter().enumerate() {
            self.line_indent();
            self.output.push_str(role_name(&parameter.role));
            self.output.push(' ');
            self.output.push_str(&parameter.name);
            self.output.push_str(": ");
            self.type_name(&parameter.ty);
            if index + 1 < parameters.len() {
                self.output.push(',');
            }
            self.output.push('\n');
        }
        self.indent -= 1;
        self.line_indent();
        self.output.push(')');
    }

    fn should_wrap_parameters(&self, parameters: &[crate::ast::Param]) -> bool {
        parameters.len() > 1
            && self.current_line_width() + parameters.iter().map(parameter_width).sum::<usize>()
                > 80
    }

    pub(super) fn current_line_width(&self) -> usize {
        self.output.rsplit('\n').next().map_or(0, str::len)
    }

    fn return_type(&mut self, return_type: &Option<crate::ast::ReturnType>) {
        if let Some(return_type) = return_type {
            self.output.push_str(" -> ");
            if matches!(return_type.access, crate::ast::ReturnAccess::Abs) {
                self.output.push_str("abs ");
            }
            self.type_name(&return_type.ty);
        }
    }

    pub(super) fn struct_definition(&mut self, definition: &crate::ast::StructDef) {
        self.documentation(definition.doc.as_deref());
        if definition.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("struct ");
        self.output.push_str(&definition.name);
        self.generic_parameters(&definition.generic_parameters);
        self.output.push_str(" {");
        if !definition.fields.is_empty() {
            self.output.push('\n');
            self.indent += 1;
            for field in &definition.fields {
                self.emit_comments_before(field.span.start);
                self.documentation(field.doc.as_deref());
                self.line_indent();
                self.output.push_str(match field.role {
                    crate::ast::StructFieldRole::Value => "",
                    crate::ast::StructFieldRole::Erg => "erg ",
                    crate::ast::StructFieldRole::Abs => "abs ",
                    crate::ast::StructFieldRole::Ins => "ins ",
                });
                self.output.push_str(&field.name);
                self.output.push_str(": ");
                self.type_name(&field.ty);
                self.output.push_str(",\n");
            }
            self.indent -= 1;
            self.line_indent();
        }
        self.output.push('}');
    }

    pub(super) fn enum_definition(&mut self, definition: &crate::ast::EnumDef) {
        self.documentation(definition.doc.as_deref());
        if definition.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("enum ");
        self.output.push_str(&definition.name);
        self.generic_parameters(&definition.generic_parameters);
        self.output.push_str(" {");
        if !definition.variants.is_empty() {
            self.output.push('\n');
            self.indent += 1;
            for variant in &definition.variants {
                self.emit_comments_before(variant.span.start);
                self.documentation(variant.doc.as_deref());
                self.line_indent();
                self.enum_variant(variant);
                self.output.push_str(",\n");
            }
            self.indent -= 1;
            self.line_indent();
        }
        self.output.push('}');
    }

    fn enum_variant(&mut self, variant: &crate::ast::EnumVariant) {
        self.output.push_str(&variant.name);
        match &variant.payload {
            crate::ast::EnumPayload::Unit => {}
            crate::ast::EnumPayload::Tuple(types) => self.enum_tuple_payload(types),
            crate::ast::EnumPayload::Struct(fields) => self.enum_struct_payload(fields),
        }
    }

    fn enum_tuple_payload(&mut self, types: &[crate::ast::TypeName]) {
        self.output.push('(');
        for (index, ty) in types.iter().enumerate() {
            if index > 0 {
                self.output.push_str(", ");
            }
            self.type_name(ty);
        }
        self.output.push(')');
    }

    fn enum_struct_payload(&mut self, fields: &[crate::ast::EnumField]) {
        self.output.push_str(" {");
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&field.name);
            self.output.push_str(": ");
            self.type_name(&field.ty);
        }
        self.output.push('}');
    }

    pub(super) fn pack_definition(&mut self, definition: &crate::ast::PackDecl) {
        self.documentation(definition.doc.as_deref());
        if definition.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("pack ");
        self.output.push_str(&definition.name);
        self.output.push_str(" { erg storage: ");
        self.type_name(&definition.storage);
        self.output.push_str("; layout ");
        self.output.push_str(match definition.endianness {
            crate::ast::LayoutEndianness::Little => "little",
            crate::ast::LayoutEndianness::Big => "big",
        });
        self.output.push_str("; fields {");
        for field in &definition.fields {
            self.emit_comments_before(field.span.start);
            self.documentation(field.doc.as_deref());
            self.output.push(' ');
            self.output.push_str(role_name(&field.role));
            self.output.push(' ');
            self.output.push_str(&field.name);
            self.output.push_str(": ");
            self.type_name(&field.ty);
            self.output.push_str(" at ");
            self.output.push_str(&field.offset.to_string());
            if let Some(default_value) = &field.default_value {
                self.output.push_str(" = ");
                self.expression(default_value);
            }
            self.output.push(';');
        }
        self.output.push_str(" } }");
    }

    pub(super) fn open_sibling(&mut self, sibling: &crate::ast::OpenSiblingDecl) {
        self.documentation(sibling.doc.as_deref());
        self.output.push_str("open ");
        self.output.push_str(&sibling.name);
        self.output.push(';');
    }

    pub(super) fn import(&mut self, import: &crate::ast::ImportDecl) {
        self.output.push_str("import ");
        self.output.push_str(&import.path);
        self.output.push(';');
    }
}

fn parameter_width(parameter: &crate::ast::Param) -> usize {
    role_name(&parameter.role).len() + parameter.name.len() + 4 + type_name_width(&parameter.ty)
}

fn type_name_width(type_name: &crate::ast::TypeName) -> usize {
    let role_width = type_name.reference_role.as_ref().map_or(0, |role| role_name(role).len() + 1);
    role_width
        + type_name.name.len()
        + if type_name.arguments.is_empty() {
            0
        } else {
            2 + type_name.arguments.iter().map(type_name_width).sum::<usize>()
                + 2 * type_name.arguments.len().saturating_sub(1)
        }
}
