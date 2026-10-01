use crate::ast::{Argument, Expr};

use super::{Formatter, role_name};

impl Formatter<'_> {
    pub(super) fn expression(&mut self, expression: &Expr) {
        match expression {
            Expr::Identifier { name, .. } => self.output.push_str(&normalize_generic_spacing(name)),
            Expr::BufferLiteral { length, .. } => {
                self.output.push_str("Buffer[");
                self.expression(length);
                self.output.push(']');
            }
            Expr::Integer { value, suffix, .. } => {
                self.output.push_str(value);
                if let Some(suffix) = suffix {
                    self.output.push_str(suffix);
                }
            }
            Expr::FloatLiteral { value, suffix, .. } => {
                self.output.push_str(value);
                if let Some(suffix) = suffix {
                    self.output.push_str(suffix);
                }
            }
            Expr::StringLiteral { value, .. } => self.string_literal(value),
            Expr::Grouping { expression, .. } => self.grouping(expression),
            Expr::Unary { operator, expression, .. } => self.unary(operator, expression),
            Expr::Cast { expression, target, .. } => {
                self.expression(expression);
                self.output.push_str(" as ");
                self.output.push_str(&target.name);
            }
            Expr::Binary { left, operator, right, .. } => self.binary(left, operator, right),
            Expr::Borrow { expression, .. } => self.borrow(expression),
            Expr::Try { expression, .. } => {
                self.expression(expression);
                self.output.push('?');
            }
            Expr::Call { callee, arguments, .. } => self.call(callee, arguments),
            Expr::MethodCall { receiver, method, arguments, .. } => {
                self.method_call(receiver, method, arguments)
            }
            Expr::StructLit { name, type_arguments, fields, .. } => {
                self.struct_literal(name, type_arguments, fields)
            }
            Expr::FieldAccess { object, field, .. } => self.field_access(object, field),
            Expr::Index { target, index, .. } => {
                self.expression(target);
                self.output.push('[');
                self.expression(index);
                self.output.push(']');
            }
            Expr::Case { mode, subject, branches, .. } => {
                self.case_expression(*mode, subject, branches)
            }
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.if_expression(condition, then_branch, else_branch.as_ref())
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

    fn unary(&mut self, operator: &crate::ast::UnaryOp, expression: &Expr) {
        self.output.push_str(match operator {
            crate::ast::UnaryOp::Negate => "-",
            crate::ast::UnaryOp::LogicalNot => "!",
            crate::ast::UnaryOp::BitwiseNot => "~",
        });
        self.expression(expression);
    }

    fn binary(&mut self, left: &Expr, operator: &crate::ast::BinaryOp, right: &Expr) {
        self.expression(left);
        self.output.push_str(match operator {
            crate::ast::BinaryOp::Add => " + ",
            crate::ast::BinaryOp::Subtract => " - ",
            crate::ast::BinaryOp::Multiply => " * ",
            crate::ast::BinaryOp::Divide => " / ",
            crate::ast::BinaryOp::Remainder => " % ",
            crate::ast::BinaryOp::BitwiseAnd => " & ",
            crate::ast::BinaryOp::BitwiseOr => " | ",
            crate::ast::BinaryOp::BitwiseXor => " ^ ",
            crate::ast::BinaryOp::ShiftLeft => " << ",
            crate::ast::BinaryOp::ShiftRight => " >> ",
            crate::ast::BinaryOp::LessThan => " < ",
            crate::ast::BinaryOp::LessEquals => " <= ",
            crate::ast::BinaryOp::GreaterThan => " > ",
            crate::ast::BinaryOp::GreaterEquals => " >= ",
            crate::ast::BinaryOp::Equals => " == ",
            crate::ast::BinaryOp::NotEquals => " != ",
            crate::ast::BinaryOp::LogicalAnd => " && ",
            crate::ast::BinaryOp::LogicalOr => " || ",
        });
        self.expression(right);
    }

    fn borrow(&mut self, expression: &Expr) {
        self.output.push_str("ref ");
        self.expression(expression);
    }

    fn call(&mut self, callee: &str, arguments: &[Argument]) {
        self.output.push_str(&normalize_generic_spacing(callee));
        if arguments.len() > 2 || self.current_line_width() > 90 {
            self.multiline_call(arguments);
            return;
        }
        self.output.push('(');
        for (index, argument) in arguments.iter().enumerate() {
            if index > 0 {
                self.output.push_str(", ");
            }
            self.argument(argument);
        }
        self.output.push(')');
    }

    fn multiline_call(&mut self, arguments: &[Argument]) {
        self.output.push_str("(\n");
        self.indent += 1;
        for (index, argument) in arguments.iter().enumerate() {
            self.line_indent();
            self.argument(argument);
            if index + 1 < arguments.len() {
                self.output.push(',');
            }
            self.output.push('\n');
        }
        self.indent -= 1;
        self.line_indent();
        self.output.push(')');
    }

    fn struct_literal(
        &mut self,
        name: &str,
        type_arguments: &[crate::ast::TypeName],
        fields: &[crate::ast::StructFieldInit],
    ) {
        self.output.push_str(name);
        if !type_arguments.is_empty() {
            self.output.push('[');
            for (index, argument) in type_arguments.iter().enumerate() {
                if index > 0 {
                    self.output.push_str(", ");
                }
                self.type_name(argument);
            }
            self.output.push(']');
        }
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
        let mode_name = match mode {
            crate::ast::CaseMode::Plain => "",
            crate::ast::CaseMode::Abs => "abs",
            crate::ast::CaseMode::Dat => "dat",
        };
        self.output.push_str(mode_name);
        if !mode_name.is_empty() {
            self.output.push(' ');
        }
        self.expression(subject);
        self.output.push_str(" {\n");
        self.indent += 1;
        for branch in branches {
            self.line_indent();
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
            self.output.push_str(",\n");
        }
        self.indent -= 1;
        self.line_indent();
        self.output.push('}');
    }

    fn if_expression(
        &mut self,
        condition: &Expr,
        then_branch: &crate::ast::Block,
        else_branch: Option<&crate::ast::IfBranch>,
    ) {
        self.output.push_str("if ");
        self.expression(condition);
        self.output.push(' ');
        self.block(then_branch);
        if let Some(else_branch) = else_branch {
            self.output.push_str(" else ");
            match else_branch {
                crate::ast::IfBranch::Block(block) => self.block(block),
                crate::ast::IfBranch::ElseIf(expression) => self.expression(expression),
            }
        }
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

fn normalize_generic_spacing(callee: &str) -> String {
    let mut normalized = String::with_capacity(callee.len() + 4);
    let mut generic_depth = 0usize;
    let mut previous_was_space = false;
    for character in callee.chars() {
        match character {
            '[' => {
                generic_depth += 1;
                normalized.push(character);
                previous_was_space = false;
            }
            ']' => {
                generic_depth = generic_depth.saturating_sub(1);
                normalized.push(character);
                previous_was_space = false;
            }
            ',' if generic_depth > 0 => {
                normalized.push(',');
                normalized.push(' ');
                previous_was_space = true;
            }
            ' ' if generic_depth > 0 && previous_was_space => {}
            _ => {
                normalized.push(character);
                previous_was_space = character == ' ';
            }
        }
    }
    normalized
}
