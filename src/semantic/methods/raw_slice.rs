use crate::ast::{Argument, BuiltinType, Expr};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::dispatch::expression_span;

impl Analyzer {
    pub(super) fn visit_raw_slice_call(
        &mut self,
        receiver: &Expr,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(receiver)?;
        if self.expression_type(receiver) != Some(BuiltinType::Buffer) || arguments.len() != 2 {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: "raw_slice".to_owned(),
                    parameter: "receiver and bounds".to_owned(),
                },
                span,
            });
        }
        for argument in arguments {
            self.visit_expression(&argument.expression)?;
            if self.expression_type(&argument.expression) != Some(BuiltinType::Int) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::InvalidIntrinsicArgument {
                        callee: "raw_slice".to_owned(),
                        parameter: "start and length".to_owned(),
                    },
                    span: expression_span(&argument.expression),
                });
            }
        }
        Ok(())
    }
}
