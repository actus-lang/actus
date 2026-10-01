use crate::ast::{Program, Role, TopLevelDecl};
use crate::lexer::SourceSpan;

mod declarations;
mod expressions;
mod statements;

pub fn format_program(program: &Program) -> String {
    format_program_with_comments(program, &[])
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceComment {
    pub span: SourceSpan,
    pub text: String,
}

pub fn format_program_with_comments(program: &Program, comments: &[SourceComment]) -> String {
    let mut formatter = Formatter {
        output: String::new(),
        indent: 0,
        comments,
        next_comment: 0,
        render_ast_docs: comments.is_empty(),
    };
    formatter.program(program);
    formatter.emit_remaining_comments();
    if !formatter.output.is_empty() {
        formatter.output.push('\n');
    }
    formatter.output
}

struct Formatter<'comments> {
    output: String,
    indent: usize,
    comments: &'comments [SourceComment],
    next_comment: usize,
    render_ast_docs: bool,
}

impl Formatter<'_> {
    fn program(&mut self, program: &Program) {
        for scope in &program.file_metadata {
            if matches!(scope, crate::ast::LimitlessScope::File) {
                self.output.push_str("meta limitless(\"file\")\n");
            }
        }
        for (index, declaration) in program.declarations.iter().enumerate() {
            if index > 0 || !program.file_metadata.is_empty() {
                self.output.push('\n');
                self.output.push('\n');
            }
            self.emit_comments_before(declaration_span(declaration).start);
            self.top_level(declaration);
        }
    }

    fn top_level(&mut self, declaration: &TopLevelDecl) {
        match declaration {
            TopLevelDecl::Constant(constant) => self.constant(constant),
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

    fn constant(&mut self, constant: &crate::ast::ConstantDecl) {
        self.documentation(constant.doc.as_deref());
        self.line_indent();
        if constant.is_open {
            self.output.push_str("open ");
        }
        self.output.push_str("const ");
        self.output.push_str(&constant.name);
        self.output.push_str(": ");
        self.type_name(&constant.ty);
        self.output.push_str(" = ");
        self.expression(&constant.initializer);
        self.output.push_str(";\n");
    }

    pub(super) fn line_indent(&mut self) {
        self.output.push_str(&"    ".repeat(self.indent));
    }

    pub(super) fn documentation(&mut self, doc: Option<&str>) {
        if !self.render_ast_docs {
            return;
        }
        let Some(doc) = doc else { return };
        let lines = doc.lines().collect::<Vec<_>>();
        self.line_indent();
        if lines.len() == 1 {
            self.output.push_str("\"\"\"");
            self.output.push_str(lines[0]);
            self.output.push_str("\"\"\"\n");
            return;
        }
        self.output.push_str("\"\"\"\n");
        for line in lines {
            self.line_indent();
            self.output.push_str(line);
            self.output.push('\n');
        }
        self.line_indent();
        self.output.push_str("\"\"\"\n");
    }

    pub(super) fn type_name(&mut self, type_name: &crate::ast::TypeName) {
        if let Some(role) = &type_name.reference_role {
            self.output.push_str(role_name(role));
            self.output.push(' ');
        }
        self.output.push_str(&type_name.name);
        if !type_name.arguments.is_empty() {
            self.output.push('[');
            for (index, argument) in type_name.arguments.iter().enumerate() {
                if index > 0 {
                    self.output.push_str(", ");
                }
                self.type_name(argument);
            }
            self.output.push(']');
        }
    }

    pub(super) fn generic_parameters(&mut self, parameters: &[crate::ast::GenericParam]) {
        if parameters.is_empty() {
            return;
        }
        self.output.push('[');
        for (index, parameter) in parameters.iter().enumerate() {
            if index > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&parameter.name);
            if !parameter.bounds.is_empty() {
                self.output.push_str(": ");
                for (bound_index, bound) in parameter.bounds.iter().enumerate() {
                    if bound_index > 0 {
                        self.output.push_str(" + ");
                    }
                    self.type_name(bound);
                }
            }
        }
        self.output.push(']');
    }

    pub(super) fn emit_comments_before(&mut self, source_start: usize) {
        while self.next_comment < self.comments.len()
            && self.comments[self.next_comment].span.start < source_start
        {
            let text = self.comments[self.next_comment].text.clone();
            if !self.output.is_empty() && !self.output.ends_with('\n') {
                self.output.push('\n');
            }
            self.line_indent();
            self.output.push_str(text.trim_end_matches('\r'));
            self.output.push('\n');
            self.next_comment += 1;
        }
    }

    fn emit_remaining_comments(&mut self) {
        while self.next_comment < self.comments.len() {
            let text = self.comments[self.next_comment].text.clone();
            if !self.output.is_empty() && !self.output.ends_with('\n') {
                self.output.push('\n');
            }
            self.line_indent();
            self.output.push_str(text.trim_end_matches('\r'));
            self.output.push('\n');
            self.next_comment += 1;
        }
    }
}

fn declaration_span(declaration: &TopLevelDecl) -> SourceSpan {
    match declaration {
        TopLevelDecl::Constant(value) => value.span,
        TopLevelDecl::Verb(value) => value.span,
        TopLevelDecl::ExternalVerb(value) => value.span,
        TopLevelDecl::Struct(value) => value.span,
        TopLevelDecl::Pack(value) => value.span,
        TopLevelDecl::Enum(value) => value.span,
        TopLevelDecl::Role(value) => value.span,
        TopLevelDecl::Perform(value) => value.span,
        TopLevelDecl::OpenSibling(value) => value.span,
        TopLevelDecl::Import(value) => value.span,
    }
}

fn meta_name(metadata: &crate::ast::MetaAttribute) -> String {
    match metadata {
        crate::ast::MetaAttribute::Test => "test".to_owned(),
        crate::ast::MetaAttribute::Target(selector) => format!("target(\"{selector}\")"),
        crate::ast::MetaAttribute::Limitless(scope) => {
            let scope = match scope {
                crate::ast::LimitlessScope::Verb => "verb",
                crate::ast::LimitlessScope::File => "file",
            };
            format!("limitless(\"{scope}\")")
        }
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
