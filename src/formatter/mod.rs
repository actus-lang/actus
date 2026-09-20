use crate::ast::{Argument, Block, Expr, Program, Role, Stmt, TopLevelDecl};

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
            TopLevelDecl::Verb(verb) => {
                self.output.push_str("verb ");
                self.output.push_str(&verb.name);
                self.output.push('(');
                for (index, parameter) in verb.params.iter().enumerate() {
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
                if let Some(return_type) = &verb.return_type {
                    self.output.push_str(" -> ");
                    self.output.push_str(&return_type.name);
                }
                self.output.push(' ');
                self.block(&verb.body);
            }
        }
    }

    fn block(&mut self, block: &Block) {
        self.output.push('{');
        if block.statements.is_empty() {
            self.output.push('}');
            return;
        }

        self.output.push('\n');
        self.indent += 1;
        for statement in &block.statements {
            self.line_indent();
            self.statement(statement);
            self.output.push('\n');
        }
        self.indent -= 1;
        self.line_indent();
        self.output.push('}');
    }

    fn statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::OwnerDecl { role, name, ty, initializer, .. } => {
                self.output.push_str(role_name(role));
                self.output.push(' ');
                self.output.push_str(name);
                if let Some(ty) = ty {
                    self.output.push_str(": ");
                    self.output.push_str(ty);
                }
                self.output.push_str(" = ");
                self.expression(initializer);
                self.output.push(';');
            }
            Stmt::Assignment { name, value, .. } => {
                self.output.push_str(name);
                self.output.push_str(" = ");
                self.expression(value);
                self.output.push(';');
            }
            Stmt::Expression { expression, .. } => {
                self.expression(expression);
                self.output.push(';');
            }
            Stmt::Return { value, .. } => {
                self.output.push_str("return");
                if let Some(value) = value {
                    self.output.push(' ');
                    self.expression(value);
                }
                self.output.push(';');
            }
            Stmt::Loop(block) => {
                self.output.push_str("loop ");
                self.block(block);
            }
            Stmt::Break { .. } => self.output.push_str("break;"),
            Stmt::Continue { .. } => self.output.push_str("continue;"),
            Stmt::Drop { name, .. } => {
                self.output.push_str("drop(");
                self.output.push_str(name);
                self.output.push_str(");");
            }
            Stmt::Block(block) => self.block(block),
        }
    }

    fn expression(&mut self, expression: &Expr) {
        match expression {
            Expr::Identifier { name, .. } => self.output.push_str(name),
            Expr::Integer { value, .. } => self.output.push_str(value),
            Expr::StringLiteral { value, .. } => {
                self.output.push('"');
                self.output.push_str(value);
                self.output.push('"');
            }
            Expr::Borrow { expression, .. } => {
                self.output.push_str("ref ");
                self.expression(expression);
            }
            Expr::Call { callee, arguments, .. } => {
                self.output.push_str(callee);
                self.output.push('(');
                for (index, argument) in arguments.iter().enumerate() {
                    if index > 0 {
                        self.output.push_str(", ");
                    }
                    self.argument(argument);
                }
                self.output.push(')');
            }
        }
    }

    fn argument(&mut self, argument: &Argument) {
        if let Some(name) = &argument.name {
            self.output.push_str(name);
            self.output.push_str(": ");
        }
        self.expression(&argument.expression);
    }

    fn line_indent(&mut self) {
        self.output.push_str(&"    ".repeat(self.indent));
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
    }
}
