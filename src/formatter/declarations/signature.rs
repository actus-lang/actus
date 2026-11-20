//! Verb signatures: parameters and return type, wrapped at the target line
//! width (docs/language/style-and-conventions.md).

use super::super::{Formatter, LINE_WIDTH, role_name};
use super::widths::parameter_width;

impl Formatter<'_> {
    /// Parameters and return type. A signature whose line, with the
    /// `terminator_width` columns that follow it, would pass the target width
    /// places one parameter per line.
    pub(super) fn signature(
        &mut self,
        parameters: &[crate::ast::Param],
        return_type: &Option<crate::ast::ReturnType>,
        terminator_width: usize,
    ) {
        let start = self.output.len();
        self.parameters(parameters);
        self.return_type(return_type);
        let single_line = !self.output[start..].contains('\n');
        if single_line
            && !parameters.is_empty()
            && self.width_of_line_at(start) + terminator_width > LINE_WIDTH
        {
            self.output.truncate(start);
            self.multiline_parameters(parameters);
            self.return_type(return_type);
        }
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

    pub(in crate::formatter) fn current_line_width(&self) -> usize {
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
}
