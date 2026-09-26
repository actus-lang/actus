use crate::ast::{Argument, Expr};

use super::{Formatter, role_name};

impl Formatter {
    pub(super) fn expression(&mut self, expression: &Expr) {
        match expression {
            Expr::Identifier { name, .. } => self.output.push_str(name),
            Expr::BufferLiteral { length, .. } => {
                self.output.push_str("Buffer[");
                self.expression(length);
                self.output.push(']');
            }
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
            Expr::Case { mode, subject, branches, .. } => {
                self.case_expression(*mode, subject, branches)
            }
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

    fn case_expression(
        &mut self,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[crate::ast::CaseBranch],
    ) {
        self.output.push_str("case ");
        self.output.push_str(match mode {
            crate::ast::CaseMode::Abs => "abs",
            crate::ast::CaseMode::Dat => "dat",
        });
        self.output.push(' ');
        self.expression(subject);
        self.output.push_str(" {");
        for branch in branches {
            self.output.push(' ');
            self.pattern(&branch.pattern);
            if let Some(guard) = &branch.guard {
                self.output.push_str(" if ");
                self.expression(guard);
            }
            self.output.push_str(" => ");
            match &branch.body {
                crate::ast::CaseBody::Expression(expression) => self.expression(expression),
                crate::ast::CaseBody::Block(block) => self.block(block),
            }
            self.output.push(',');
        }
        self.output.push_str(" }");
    }

    fn pattern(&mut self, pattern: &crate::ast::Pattern) {
        match pattern {
            crate::ast::Pattern::Wildcard { .. } => self.output.push('_'),
            crate::ast::Pattern::Literal { value, .. } => match value {
                crate::ast::LiteralPattern::Integer(value) => self.output.push_str(value),
                crate::ast::LiteralPattern::Bool(value) => {
                    self.output.push_str(if *value { "true" } else { "false" });
                }
            },
            crate::ast::Pattern::Variant { enum_name, variant, payload, .. } => {
                self.output.push_str(enum_name);
                self.output.push('.');
                self.output.push_str(variant);
                self.pattern_payload(payload);
            }
        }
    }

    fn pattern_payload(&mut self, payload: &crate::ast::VariantPayload) {
        match payload {
            crate::ast::VariantPayload::Unit => {}
            crate::ast::VariantPayload::Positional(bindings) => {
                self.output.push('(');
                for (index, binding) in bindings.iter().enumerate() {
                    if index > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&binding.name);
                }
                self.output.push(')');
            }
            crate::ast::VariantPayload::Named(fields) => {
                self.output.push_str(" {");
                for (index, field) in fields.iter().enumerate() {
                    if index > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&field.name);
                    self.output.push_str(": ");
                    self.output.push_str(&field.binding.name);
                }
                self.output.push('}');
            }
        }
    }

    fn argument(&mut self, argument: &Argument) {
        if let Some(name) = &argument.name {
            self.output.push_str(name);
            self.output.push_str(": ");
        }
        if let Some(role) = &argument.role {
            self.output.push_str(role_name(role));
            self.output.push(' ');
        }
        self.expression(&argument.expression);
    }
}
