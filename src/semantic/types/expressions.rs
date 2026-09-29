use crate::ast::{
    BuiltinType, CaseBody, Expr, IntrinsicKind, Stmt, lookup_builtin_type, lookup_call_intrinsic,
};

use super::super::analyzer::Analyzer;

impl Analyzer {
    pub(crate) fn expression_type(&self, expression: &Expr) -> Option<BuiltinType> {
        match expression {
            Expr::Integer { .. } => Some(BuiltinType::Int),
            Expr::BufferLiteral { .. } => Some(BuiltinType::Buffer),
            Expr::FloatLiteral { .. } => None,
            Expr::StringLiteral { .. } => Some(BuiltinType::String),
            Expr::Grouping { expression, .. }
            | Expr::Borrow { expression, .. }
            | Expr::Unary { expression, .. } => self.expression_type(expression),
            Expr::Cast { target, .. } => lookup_builtin_type(&target.name),
            Expr::Try { expression, .. } => self
                .enum_type_application(expression)
                .and_then(|type_name| type_name.arguments.first().cloned())
                .and_then(|type_name| lookup_builtin_type(&type_name.name)),
            Expr::Binary { operator, .. } if operator.is_relational() => Some(BuiltinType::Bool),
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
        }
    }

    fn call_expression_type(
        &self,
        callee: &str,
        span: crate::lexer::SourceSpan,
    ) -> Option<BuiltinType> {
        match lookup_call_intrinsic(callee) {
            Some(IntrinsicKind::Append | IntrinsicKind::Print) => Some(BuiltinType::Int),
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

    pub(crate) fn expression_type_name(&self, expression: &Expr) -> Option<String> {
        if matches!(expression, Expr::FloatLiteral { .. }) {
            return Some("f64".to_owned());
        }
        if let Expr::Call { span, .. } | Expr::MethodCall { span, .. } = expression
            && let Some(type_name) = self.inferred_expression_types.get(&(span.start, span.end))
        {
            return Some(super::super::analyzer::canonical_type_name(type_name));
        }
        if let Expr::Cast { target, .. } = expression {
            return Some(super::super::analyzer::canonical_type_name(target));
        }
        self.binding_type_name(expression)
            .or_else(|| self.index_type_name(expression))
            .or_else(|| self.expression_type(expression).map(|ty| ty.spec().name.to_owned()))
            .or_else(|| self.expression_struct_type(expression))
            .or_else(|| self.expression_pack_type(expression))
            .or_else(|| {
                self.resolved_type_name(expression)
                    .map(|name| super::super::analyzer::canonical_type_name(&name))
            })
            .or_else(|| self.expression_enum_type_application(expression))
            .or_else(|| self.expression_case_type_name(expression))
            .or_else(|| self.expression_enum_type(expression))
    }

    fn index_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Index { target, index, .. } = expression else { return None };
        self.validate_index_access(target, index)
            .ok()
            .map(|type_name| super::super::analyzer::canonical_type_name(&type_name))
    }

    fn binding_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Identifier { name, span } = expression else { return None };
        let index = self.binding(name, *span).ok()?;
        self.binding_type_names.get(&index).map(super::super::analyzer::canonical_type_name)
    }

    fn expression_case_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Case { branches, .. } = expression else { return None };
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
}
