use crate::ast::{Program, Role, TopLevelDecl};

mod declarations;
mod expressions;
mod statements;

pub fn format_program(program: &Program) -> String {
    let mut formatter = Formatter { output: String::new(), indent: 0 };
    formatter.program(program);
    if !formatter.output.is_empty() {
        formatter.output.push('\n');
    }
    formatter.output
}

struct Formatter {
    output: String,
    indent: usize,
}

impl Formatter {
    fn program(&mut self, program: &Program) {
        for (index, declaration) in program.declarations.iter().enumerate() {
            if index > 0 {
                self.output.push('\n');
            }
            self.top_level(declaration);
        }
    }

    fn top_level(&mut self, declaration: &TopLevelDecl) {
        match declaration {
            TopLevelDecl::Verb(verb) => self.verb(verb),
            TopLevelDecl::ExternalVerb(verb) => self.external_verb(verb),
            TopLevelDecl::Struct(definition) => self.struct_definition(definition),
            TopLevelDecl::Pack(definition) => self.pack_definition(definition),
            TopLevelDecl::Enum(definition) => self.enum_definition(definition),
            TopLevelDecl::Role(role) => self.role_definition(role),
            TopLevelDecl::Perform(perform) => self.perform_definition(perform),
            TopLevelDecl::OpenSibling(sibling) => self.open_sibling(sibling),
            TopLevelDecl::Import(import) => self.import(import),
        }
    }

    pub(super) fn line_indent(&mut self) {
        self.output.push_str(&"    ".repeat(self.indent));
    }
}

fn meta_name(metadata: &crate::ast::MetaAttribute) -> String {
    match metadata {
        crate::ast::MetaAttribute::Test => "test".to_owned(),
        crate::ast::MetaAttribute::Target(selector) => format!("target(\"{selector}\")"),
    }
}

pub(super) fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}
