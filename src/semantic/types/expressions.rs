use crate::ast::{
    BuiltinType, CaseBody, Expr, IntrinsicKind, Stmt, lookup_builtin_type, lookup_call_intrinsic,
    primitive_type,
};

use super::super::analyzer::Analyzer;
use super::super::analyzer::expression_span;

impl Analyzer {
    pub(crate) fn expression_type(&self, expression: &Expr) -> Option<BuiltinType> {
        match expression {
            Expr::BoolLiteral { .. } => Some(BuiltinType::Bool),
            Expr::Integer { .. } => Some(BuiltinType::Int),
            Expr::BufferLiteral { .. } => Some(BuiltinType::Buffer),
            Expr::FloatLiteral { .. } => None,
            Expr::StringLiteral { .. } => Some(BuiltinType::String),
            Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
                self.expression_type(expression)
            }
            Expr::Unary { operator, expression, .. } => {
                if matches!(operator, crate::ast::UnaryOp::LogicalNot) {
                    Some(BuiltinType::Bool)
                } else {
                    self.expression_type(expression)
                }
            }
            Expr::Cast { target, .. } => lookup_builtin_type(&target.name),
            Expr::Try { expression, .. } => self
                .enum_type_application(expression)
                .and_then(|type_name| type_name.arguments.first().cloned())
                .and_then(|type_name| lookup_builtin_type(&type_name.name)),
            Expr::Binary {
                operator:
                    crate::ast::BinaryOp::Equals
                    | crate::ast::BinaryOp::NotEquals
                    | crate::ast::BinaryOp::LogicalAnd
                    | crate::ast::BinaryOp::LogicalOr
                    | crate::ast::BinaryOp::LessThan
                    | crate::ast::BinaryOp::LessEquals
                    | crate::ast::BinaryOp::GreaterThan
                    | crate::ast::BinaryOp::GreaterEquals,
                ..
            } => Some(BuiltinType::Bool),
            Expr::Binary { .. } => Some(BuiltinType::Int),
            Expr::Identifier { name, span } => {
                self.binding(name, *span).ok().and_then(|index| self.model.bindings[index].ty)
            }
            Expr::Call { callee, span, .. } => self.call_expression_type(callee, *span),
            Expr::MethodCall { receiver, method, .. } if method == "raw_slice" => {
                (self.expression_type(receiver) == Some(BuiltinType::Buffer))
                    .then_some(BuiltinType::Buffer)
            }
            Expr::MethodCall { method, span, .. } => self.method_expression_type(method, *span),
            Expr::StructLit { .. } => None,
            Expr::FieldAccess { object, field, .. } => self.field_expression_type(object, field),
            Expr::Index { target, index, .. } => self
                .validate_index_access(target, index)
                .ok()
                .and_then(|type_name| lookup_builtin_type(&type_name.name)),
            Expr::Case { branches, .. } => self.case_expression_type(branches),
            Expr::If { then_branch, else_branch, .. } => {
                self.if_expression_type(then_branch, else_branch.as_ref())
            }
        }
    }

    fn call_expression_type(
        &self,
        callee: &str,
        span: crate::lexer::SourceSpan,
    ) -> Option<BuiltinType> {
        match lookup_call_intrinsic(callee) {
            Some(
                IntrinsicKind::Append
                | IntrinsicKind::BufferLength
                | IntrinsicKind::Crc32
                | IntrinsicKind::Crc32Matches
                | IntrinsicKind::ValidateFixedFrame
                | IntrinsicKind::Print,
            ) => Some(BuiltinType::Int),
            Some(IntrinsicKind::Copy) => self
                .inferred_expression_types
                .get(&(span.start, span.end))
                .and_then(|type_name| lookup_builtin_type(&type_name.name)),
            Some(IntrinsicKind::Drop) => None,
            None => self
                .inferred_expression_types
                .get(&(span.start, span.end))
                .and_then(|type_name| lookup_builtin_type(&type_name.name))
                .or_else(|| {
                    self.signatures.get(callee).and_then(|signature| signature.return_type)
                }),
        }
    }

    fn method_expression_type(
        &self,
        method: &str,
        span: crate::lexer::SourceSpan,
    ) -> Option<BuiltinType> {
        self.inferred_expression_types
            .get(&(span.start, span.end))
            .and_then(|type_name| lookup_builtin_type(&type_name.name))
            .or_else(|| self.signatures.get(method).and_then(|signature| signature.return_type))
    }

    fn field_expression_type(&self, object: &Expr, field: &str) -> Option<BuiltinType> {
        self.expression_struct_type(object)
            .and_then(|name| self.struct_field(&name, field))
            .and_then(|field| lookup_builtin_type(&field.ty.name))
    }

    fn case_expression_type(&self, branches: &[crate::ast::CaseBranch]) -> Option<BuiltinType> {
        branches.iter().find_map(|branch| match &branch.body {
            CaseBody::Expression(expression) => self.expression_type(expression),
            CaseBody::Block(_) => None,
        })
    }

    fn if_expression_type(
        &self,
        then_branch: &crate::ast::Block,
        else_branch: Option<&crate::ast::IfBranch>,
    ) -> Option<BuiltinType> {
        let then_type = block_tail_builtin_type(self, then_branch);
        let else_type = else_branch.as_ref().and_then(|branch| match branch {
            crate::ast::IfBranch::Block(block) => block_tail_builtin_type(self, block),
            crate::ast::IfBranch::ElseIf(expression) => self.expression_type(expression),
        });
        if then_type == else_type {
            return then_type;
        }
        if then_type.is_none() && block_diverges(then_branch) {
            return else_type;
        }
        if else_type.is_none()
            && else_branch.as_ref().is_some_and(|branch| self.if_branch_diverges(branch))
        {
            return then_type;
        }
        None
    }

    pub(crate) fn expression_type_name(&self, expression: &Expr) -> Option<String> {
        if let Some(type_name) = self.literal_type_name(expression) {
            return Some(type_name);
        }
        if let Expr::Identifier { name, .. } = expression
            && self.is_const_generic_parameter(name)
        {
            return Some("Usize".to_owned());
        }
        if let Some(type_name) = self.unary_type_name(expression) {
            return Some(type_name);
        }
        if let Some(type_name) = self.binary_type_name(expression) {
            return Some(type_name);
        }
        if let Expr::Call { callee, span, .. } = expression {
            if let Some(type_name) = self.inferred_expression_types.get(&(span.start, span.end)) {
                return Some(super::super::analyzer::canonical_type_name(type_name));
            }
            if let Some(type_name) = self
                .signatures
                .get(callee)
                .and_then(|signature| signature.return_type_name.as_ref())
            {
                return Some(super::super::analyzer::canonical_type_name(type_name));
            }
        }
        if let Expr::MethodCall { method, span, .. } = expression {
            if let Some(type_name) = self.inferred_expression_types.get(&(span.start, span.end)) {
                return Some(super::super::analyzer::canonical_type_name(type_name));
            }
            if let Some(type_name) = self
                .signatures
                .get(method)
                .and_then(|signature| signature.return_type_name.as_ref())
            {
                return Some(super::super::analyzer::canonical_type_name(type_name));
            }
        }
        if let Expr::Cast { target, .. } = expression {
            return Some(super::super::analyzer::canonical_type_name(target));
        }
        self.binding_type_name(expression)
            .or_else(|| self.constant_type_name(expression))
            .or_else(|| self.index_type_name(expression))
            .or_else(|| self.field_type_name(expression))
            .or_else(|| self.expression_case_type_name(expression))
            .or_else(|| self.expression_if_type_name(expression))
            .or_else(|| self.expression_type(expression).map(|ty| ty.spec().name.to_owned()))
            .or_else(|| self.expression_struct_type(expression))
            .or_else(|| self.expression_pack_type(expression))
            .or_else(|| {
                self.resolved_type_name(expression)
                    .map(|name| super::super::analyzer::canonical_type_name(&name))
            })
            .or_else(|| self.expression_enum_type_application(expression))
            .or_else(|| self.expression_enum_type(expression))
    }

    fn literal_type_name(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::BoolLiteral { .. } => Some("Bool".to_owned()),
            Expr::Integer { suffix: Some(suffix), .. }
                if matches!(
                    primitive_type(suffix),
                    Some(crate::ast::PrimitiveType::Integer { .. })
                ) =>
            {
                Some(suffix.clone())
            }
            Expr::FloatLiteral { suffix, .. } => {
                Some(suffix.as_deref().unwrap_or("f64").to_owned())
            }
            Expr::Grouping { expression, .. } => self.expression_type_name(expression),
            _ => None,
        }
    }

    fn unary_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Unary { operator, expression, .. } = expression else { return None };
        if matches!(operator, crate::ast::UnaryOp::LogicalNot) {
            Some("Bool".to_owned())
        } else {
            self.expression_type_name(expression)
                .or_else(|| self.expression_type(expression).map(|ty| ty.spec().name.to_owned()))
        }
    }

    fn binary_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Binary { operator, left, .. } = expression else { return None };
        (!matches!(
            operator,
            crate::ast::BinaryOp::Equals
                | crate::ast::BinaryOp::NotEquals
                | crate::ast::BinaryOp::LogicalAnd
                | crate::ast::BinaryOp::LogicalOr
                | crate::ast::BinaryOp::LessThan
                | crate::ast::BinaryOp::LessEquals
                | crate::ast::BinaryOp::GreaterThan
                | crate::ast::BinaryOp::GreaterEquals
        ))
        .then(|| self.expression_type_name(left))
        .flatten()
    }

    fn index_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Index { target, index, .. } = expression else { return None };
        self.validate_index_access(target, index)
            .ok()
            .map(|type_name| super::super::analyzer::canonical_type_name(&type_name))
    }

    fn field_type_name(&self, expression: &Expr) -> Option<String> {
        if !matches!(expression, Expr::FieldAccess { .. }) {
            return None;
        }
        self.resolved_type_name(expression)
            .map(|type_name| super::super::analyzer::canonical_type_name(&type_name))
    }

    fn binding_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Identifier { name, span } = expression else { return None };
        let index = self.binding(name, *span).ok()?;
        self.binding_type_names.get(&index).map(super::super::analyzer::canonical_type_name)
    }

    fn constant_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Identifier { name, .. } = expression else { return None };
        self.constants.get(name).map(super::super::analyzer::canonical_type_name)
    }

    fn expression_case_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Case { branches, span, .. } = expression else { return None };
        if let Some(type_name) = self.case_result_types.get(&(span.start, span.end)) {
            return Some(super::super::analyzer::canonical_type_name(type_name));
        }
        branches.iter().find_map(|branch| match &branch.body {
            CaseBody::Expression(expression) => self.expression_type_name(expression),
            CaseBody::Block(block) => block.statements.iter().rev().find_map(|statement| {
                let Stmt::Return { value: Some(expression), .. } = statement else {
                    return None;
                };
                self.expression_type_name(expression)
            }),
        })
    }

    fn expression_if_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::If { then_branch, else_branch, .. } = expression else { return None };
        let then_type = block_tail_type_name(self, then_branch);
        let else_type = else_branch.as_ref().and_then(|branch| match branch {
            crate::ast::IfBranch::Block(block) => block_tail_type_name(self, block),
            crate::ast::IfBranch::ElseIf(expression) => self.expression_type_name(expression),
        });
        if then_type == else_type {
            return then_type;
        }
        if then_type.is_none() && block_diverges(then_branch) {
            return else_type;
        }
        if else_type.is_none()
            && else_branch.as_ref().is_some_and(|branch| self.if_branch_diverges(branch))
        {
            return then_type;
        }
        None
    }

    fn if_branch_diverges(&self, branch: &crate::ast::IfBranch) -> bool {
        match branch {
            crate::ast::IfBranch::Block(block) => block_diverges(block),
            crate::ast::IfBranch::ElseIf(expression) => {
                matches!(expression.as_ref(), Expr::If { .. })
                    && self.expression_type_name(expression).is_none()
            }
        }
    }
}

fn block_diverges(block: &crate::ast::Block) -> bool {
    matches!(
        block.statements.last(),
        Some(
            crate::ast::Stmt::Return { .. }
                | crate::ast::Stmt::Break { .. }
                | crate::ast::Stmt::Continue { .. }
        )
    )
}

fn block_tail_builtin_type(analyzer: &Analyzer, block: &crate::ast::Block) -> Option<BuiltinType> {
    let crate::ast::Stmt::Expression { expression, span } = block.statements.last()? else {
        return None;
    };
    (span.end == expression_span(expression).end)
        .then(|| analyzer.expression_type(expression))
        .flatten()
}

fn block_tail_type_name(analyzer: &Analyzer, block: &crate::ast::Block) -> Option<String> {
    let crate::ast::Stmt::Expression { expression, span } = block.statements.last()? else {
        return None;
    };
    (span.end == expression_span(expression).end)
        .then(|| analyzer.expression_type_name(expression))
        .flatten()
}
