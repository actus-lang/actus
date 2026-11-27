//! Breaking of binary operator chains that would pass the line width
//! (docs/language/style-and-conventions.md, section 7).

use crate::ast::{BinaryOp, Expr};

use super::expressions::binary_operator_text;
use super::{Formatter, LINE_WIDTH};

impl Formatter<'_> {
    /// Prints `expression` where it ends a line followed by `reserve` more columns (`;`, `,`
    /// or ` {`). A binary operator chain whose line would pass the width breaks before the
    /// operators of its outermost precedence level: one continuation per line, one level
    /// deeper than the current line, each line packed with as many operands as fit. Operands
    /// and parentheses are printed exactly as elsewhere; any other expression is unchanged.
    pub(super) fn wrapped_expression(&mut self, expression: &Expr, reserve: usize) {
        let Expr::Binary { operator, .. } = expression else {
            self.expression(expression);
            return;
        };
        let start = self.output.len();
        self.expression(expression);
        // An operand that already spans lines (a struct literal, a multi-line call) is left
        // alone: its own layout decides the line width.
        if self.output[start..].contains('\n')
            || self.width_of_line_at(start) + reserve <= LINE_WIDTH
        {
            return;
        }
        self.output.truncate(start);

        // `a - b + c` parses as `(a - b) + c`, so the chain is the left spine of operators that
        // share the root's precedence; every right operand binds tighter.
        let level = precedence(operator);
        let mut operands = Vec::new();
        let mut operators = Vec::new();
        let mut node = expression;
        while let Expr::Binary { left, operator, right, .. } = node {
            if precedence(operator) != level {
                break;
            }
            operands.push(&**right);
            operators.push(operator);
            node = left;
        }
        operands.push(node);
        operands.reverse();
        operators.reverse();

        self.expression(operands[0]);
        for (index, operand) in operands.iter().skip(1).enumerate() {
            let operator = binary_operator_text(operators[index]);
            let line_start = self.output.len();
            self.output.push_str(operator);
            self.expression(operand);
            let is_last = index + 2 == operands.len();
            let needed = if is_last { reserve } else { 0 };
            if self.width_of_line_at(line_start) + needed > LINE_WIDTH {
                self.output.truncate(line_start);
                self.output.push('\n');
                self.indent += 1;
                self.line_indent();
                self.indent -= 1;
                self.output.push_str(operator.trim_start());
                self.expression(operand);
            }
        }
    }
}

/// Binding strength of `operator`, as the parser applies it: a larger number binds tighter.
fn precedence(operator: &BinaryOp) -> u8 {
    match operator {
        BinaryOp::LogicalOr => 0,
        BinaryOp::LogicalAnd => 1,
        BinaryOp::Equals | BinaryOp::NotEquals => 2,
        BinaryOp::LessThan
        | BinaryOp::LessEquals
        | BinaryOp::GreaterThan
        | BinaryOp::GreaterEquals => 3,
        BinaryOp::BitwiseOr => 4,
        BinaryOp::BitwiseXor => 5,
        BinaryOp::BitwiseAnd => 6,
        BinaryOp::ShiftLeft | BinaryOp::ShiftRight => 7,
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::WrappingAdd
        | BinaryOp::WrappingSubtract
        | BinaryOp::SaturatingAdd
        | BinaryOp::SaturatingSubtract => 8,
        BinaryOp::Multiply
        | BinaryOp::WrappingMultiply
        | BinaryOp::SaturatingMultiply
        | BinaryOp::Divide
        | BinaryOp::Remainder => 9,
    }
}
