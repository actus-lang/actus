use super::{Formatter, meta_name, role_name};

impl Formatter {
    pub(super) fn role_definition(&mut self, role: &crate::ast::RoleDecl) {
        if role.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("role ");
        self.output.push_str(&role.name);
        self.output.push_str(" {");
        for method in &role.methods {
            self.output.push_str(" verb ");
            self.output.push_str(&method.name);
            self.parameters(&method.params);
            self.return_type(&method.return_type);
            self.output.push(';');
        }
        self.output.push_str(" }");
    }

    pub(super) fn perform_definition(&mut self, perform: &crate::ast::PerformDecl) {
        if perform.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("perform ");
        self.output.push_str(&perform.role_name);
        self.output.push_str(" for ");
        self.output.push_str(&perform.target.name);
        self.output.push_str(" {");
        for method in &perform.methods {
            self.output.push(' ');
            self.verb(method);
        }
        self.output.push_str(" }");
    }

    pub(super) fn verb(&mut self, verb: &crate::ast::VerbDecl) {
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
        self.parameters(&verb.params);
        self.return_type(&verb.return_type);
        self.output.push(' ');
        self.block(&verb.body);
    }

    pub(super) fn external_verb(&mut self, verb: &crate::ast::ExternalVerbDecl) {
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
        self.parameters(&verb.params);
        self.return_type(&verb.return_type);
        self.output.push(';');
    }

    fn parameters(&mut self, parameters: &[crate::ast::Param]) {
        self.output.push('(');
        for (index, parameter) in parameters.iter().enumerate() {
            if index > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(role_name(&parameter.role));
            self.output.push(' ');
            self.output.push_str(&parameter.name);
            self.output.push_str(": ");
            self.output.push_str(&parameter.ty.name);
        }
        self.output.push(')');
    }

    fn return_type(&mut self, return_type: &Option<crate::ast::ReturnType>) {
        if let Some(return_type) = return_type {
            self.output.push_str(" -> ");
            if matches!(return_type.access, crate::ast::ReturnAccess::Abs) {
                self.output.push_str("abs ");
            }
            self.output.push_str(&return_type.ty.name);
        }
    }

    pub(super) fn struct_definition(&mut self, definition: &crate::ast::StructDef) {
        if definition.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("struct ");
        self.output.push_str(&definition.name);
        self.output.push_str(" {");
        if !definition.fields.is_empty() {
            self.output.push('\n');
            self.indent += 1;
            for field in &definition.fields {
                self.line_indent();
                self.output.push_str(match field.role {
                    crate::ast::StructFieldRole::Value => "",
                    crate::ast::StructFieldRole::Erg => "erg ",
                    crate::ast::StructFieldRole::Abs => "abs ",
                    crate::ast::StructFieldRole::Ins => "ins ",
                });
                self.output.push_str(&field.name);
                self.output.push_str(": ");
                self.output.push_str(&field.ty.name);
                self.output.push_str(",\n");
            }
            self.indent -= 1;
            self.line_indent();
        }
        self.output.push('}');
    }

    pub(super) fn enum_definition(&mut self, definition: &crate::ast::EnumDef) {
        if definition.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("enum ");
        self.output.push_str(&definition.name);
        self.output.push_str(" {");
        if !definition.variants.is_empty() {
            self.output.push('\n');
            self.indent += 1;
            for variant in &definition.variants {
                self.line_indent();
                self.output.push_str(&variant.name);
                match &variant.payload {
                    crate::ast::EnumPayload::Unit => {}
                    crate::ast::EnumPayload::Tuple(types) => {
                        self.output.push('(');
                        for (index, ty) in types.iter().enumerate() {
                            if index > 0 {
                                self.output.push_str(", ");
                            }
                            self.output.push_str(&ty.name);
                        }
                        self.output.push(')');
                    }
                    crate::ast::EnumPayload::Struct(fields) => {
                        self.output.push_str(" {");
                        for (index, field) in fields.iter().enumerate() {
                            if index > 0 {
                                self.output.push_str(", ");
                            }
                            self.output.push_str(&field.name);
                            self.output.push_str(": ");
                            self.output.push_str(&field.ty.name);
                        }
                        self.output.push('}');
                    }
                }
                self.output.push_str(",\n");
            }
            self.indent -= 1;
            self.line_indent();
        }
        self.output.push('}');
    }

    pub(super) fn pack_definition(&mut self, definition: &crate::ast::PackDecl) {
        if definition.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("pack ");
        self.output.push_str(&definition.name);
        self.output.push_str(" { erg storage: ");
        self.output.push_str(&definition.storage.name);
        self.output.push_str("; layout ");
        self.output.push_str(match definition.endianness {
            crate::ast::LayoutEndianness::Little => "little",
            crate::ast::LayoutEndianness::Big => "big",
        });
        self.output.push_str("; fields {");
        for field in &definition.fields {
            self.output.push(' ');
            self.output.push_str(role_name(&field.role));
            self.output.push(' ');
            self.output.push_str(&field.name);
            self.output.push_str(": ");
            self.output.push_str(&field.ty.name);
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
