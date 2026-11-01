use super::Formatter;

impl Formatter<'_> {
    pub(super) fn import(&mut self, import: &crate::ast::ImportDecl) {
        self.output.push_str("import ");
        if import.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str(&import.path);
        if !import.selected.is_empty() {
            self.output.push_str(" {");
            for (index, name) in import.selected.iter().enumerate() {
                if index > 0 {
                    self.output.push_str(", ");
                }
                self.output.push_str(name);
            }
            self.output.push('}');
        }
        if let Some(alias) = &import.alias {
            self.output.push_str(" as ");
            self.output.push_str(alias);
        }
        self.output.push(';');
    }

    pub(super) fn import_group(&mut self, declarations: &[crate::ast::TopLevelDecl]) {
        self.output.push_str("import {\n");
        for declaration in declarations {
            let crate::ast::TopLevelDecl::Import(import) = declaration else {
                continue;
            };
            self.output.push_str("    ");
            if import.is_open {
                self.output.push_str("open ");
            }
            self.output.push_str(&import.path);
            if !import.selected.is_empty() {
                self.output.push_str(" {");
                for (index, name) in import.selected.iter().enumerate() {
                    if index > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(name);
                }
                self.output.push('}');
            }
            if let Some(alias) = &import.alias {
                self.output.push_str(" as ");
                self.output.push_str(alias);
            }
            self.output.push_str(",\n");
        }
        self.output.push('}');
        self.output.push(';');
    }
}
