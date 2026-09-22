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
            TopLevelDecl::Verb(verb) => self.verb(verb),
            TopLevelDecl::ExternalVerb(verb) => self.external_verb(verb),
            TopLevelDecl::Struct(definition) => self.struct_definition(definition),
            TopLevelDecl::Enum(definition) => self.enum_definition(definition),
        }
    }

    fn verb(&mut self, verb: &crate::ast::VerbDecl) {
        self.output.push_str("verb ");
        self.output.push_str(&verb.name);
        self.parameters(&verb.params);
        self.return_type(&verb.return_type);
        self.output.push(' ');
        self.block(&verb.body);
    }

    fn external_verb(&mut self, verb: &crate::ast::ExternalVerbDecl) {
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

    fn return_type(&mut self, return_type: &Option<crate::ast::TypeName>) {
        if let Some(return_type) = return_type {
            self.output.push_str(" -> ");
            self.output.push_str(&return_type.name);
        }
    }

    fn struct_definition(&mut self, definition: &crate::ast::StructDef) {
        self.output.push_str("struct ");
        self.output.push_str(&definition.name);
        self.output.push_str(" {");
        if !definition.fields.is_empty() {
            self.output.push('\n');
            self.indent += 1;
            for field in &definition.fields {
                self.line_indent();
                if matches!(field.role, crate::ast::StructFieldRole::Erg) {
                    self.output.push_str("erg ");
                }
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

    fn enum_definition(&mut self, definition: &crate::ast::EnumDef) {
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
            Stmt::FieldAssignment { object, field, value, .. } => {
                self.expression(object);
                self.output.push('.');
                self.output.push_str(field);
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
            Expr::Integer { value, .. } | Expr::FloatLiteral { value, .. } => {
                self.output.push_str(value)
            }
            Expr::StringLiteral { value, .. } => self.string_literal(value),
            Expr::Grouping { expression, .. } => self.grouping(expression),
            Expr::Unary { expression, .. } => self.unary(expression),
            Expr::Binary { left, operator, right, .. } => self.binary(left, operator, right),
            Expr::Borrow { expression, .. } => self.borrow(expression),
            Expr::Call { callee, arguments, .. } => self.call(callee, arguments),
            Expr::MethodCall { receiver, method, arguments, .. } => {
                self.method_call(receiver, method, arguments)
            }
            Expr::StructLit { name, fields, .. } => self.struct_literal(name, fields),
            Expr::FieldAccess { object, field, .. } => self.field_access(object, field),
        }
    }

    fn string_literal(&mut self, value: &str) {
        self.output.push('"');
        self.output.push_str(value);
        self.output.push('"');
    }

    fn grouping(&mut self, expression: &Expr) {
        self.output.push('(');
        self.expression(expression);
        self.output.push(')');
    }

    fn unary(&mut self, expression: &Expr) {
        self.output.push('-');
        self.expression(expression);
    }

    fn binary(&mut self, left: &Expr, operator: &crate::ast::BinaryOp, right: &Expr) {
        self.expression(left);
        self.output.push_str(match operator {
            crate::ast::BinaryOp::Add => " + ",
            crate::ast::BinaryOp::Subtract => " - ",
            crate::ast::BinaryOp::Multiply => " * ",
            crate::ast::BinaryOp::Divide => " / ",
        });
        self.expression(right);
    }

    fn borrow(&mut self, expression: &Expr) {
        self.output.push_str("ref ");
        self.expression(expression);
    }

    fn call(&mut self, callee: &str, arguments: &[Argument]) {
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

    fn struct_literal(&mut self, name: &str, fields: &[crate::ast::StructFieldInit]) {
        self.output.push_str(name);
        self.output.push_str(" {");
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&field.name);
            self.output.push_str(": ");
            self.expression(&field.value);
        }
        self.output.push('}');
    }

    fn field_access(&mut self, object: &Expr, field: &str) {
        self.expression(object);
        self.output.push('.');
        self.output.push_str(field);
    }

    fn method_call(&mut self, receiver: &Expr, method: &str, arguments: &[Argument]) {
        self.expression(receiver);
        self.output.push('.');
        self.call(method, arguments);
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
