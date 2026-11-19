//! Collects every declaration reference spelled in one source program.
//!
//! References are spellings, not resolutions: the scope check decides which
//! declaration a spelling names. Bindings local to a callable (parameters,
//! locals, loop and pattern bindings) and generic parameters are recorded so
//! that a local never reads as a reference to a module declaration.

use std::collections::HashSet;

use crate::ast::{
    Block, CaseBody, EnumPayload, Expr, GenericParam, IfBranch, Param, Pattern, Place, Program,
    ReturnType, Stmt, TopLevelDecl, TypeName, VariantPayload,
};
use crate::lexer::SourceSpan;

/// One path spelling such as `Holder`, `runtime::count`, or `Shape::Circle`,
/// with any generic application removed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Reference {
    pub(super) path: String,
    pub(super) span: SourceSpan,
}

pub(super) fn program_references(program: &Program) -> Vec<Reference> {
    let mut collector = Collector::default();
    for declaration in &program.declarations {
        collector.declaration(declaration);
    }
    collector.references
}

#[derive(Default)]
struct Collector {
    references: Vec<Reference>,
    locals: HashSet<String>,
}

impl Collector {
    fn declaration(&mut self, declaration: &TopLevelDecl) {
        self.locals.clear();
        match declaration {
            TopLevelDecl::Constant(value) => {
                self.type_name(&value.ty);
                self.expr(&value.initializer);
            }
            TopLevelDecl::Verb(value) => {
                self.callable(&value.generic_parameters, &value.params, &value.return_type);
                collect_block_bindings(&value.body, &mut self.locals);
                self.block(&value.body);
            }
            TopLevelDecl::ExternalVerb(value) => {
                self.callable(&value.generic_parameters, &value.params, &value.return_type)
            }
            TopLevelDecl::Struct(value) => {
                self.generics(&value.generic_parameters);
                value.fields.iter().for_each(|field| self.type_name(&field.ty));
            }
            TopLevelDecl::Enum(value) => {
                self.generics(&value.generic_parameters);
                value.variants.iter().for_each(|variant| self.payload(&variant.payload));
            }
            TopLevelDecl::Pack(value) => self.pack(value),
            TopLevelDecl::Serialize(value) => self.serialize(value),
            TopLevelDecl::Role(value) => {
                for method in &value.methods {
                    self.callable(&[], &method.params, &method.return_type);
                }
            }
            TopLevelDecl::Perform(value) => self.perform(value),
            TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_) => {}
        }
    }

    fn callable(
        &mut self,
        generics: &[GenericParam],
        params: &[Param],
        return_type: &Option<ReturnType>,
    ) {
        self.generics(generics);
        for param in params {
            self.locals.insert(param.name.clone());
            self.type_name(&param.ty);
        }
        if let Some(return_type) = return_type {
            self.type_name(&return_type.ty);
        }
    }

    fn generics(&mut self, generics: &[GenericParam]) {
        for parameter in generics {
            self.locals.insert(parameter.name.clone());
        }
        for parameter in generics {
            parameter.bound.iter().chain(&parameter.bounds).for_each(|bound| self.type_name(bound));
            if let crate::ast::GenericParamKind::Const { domain } = &parameter.kind {
                self.type_name(domain);
            }
        }
    }

    fn payload(&mut self, payload: &EnumPayload) {
        match payload {
            EnumPayload::Tuple(types) => types.iter().for_each(|ty| self.type_name(ty)),
            EnumPayload::Struct(fields) => {
                fields.iter().for_each(|field| self.type_name(&field.ty))
            }
            EnumPayload::Unit => {}
        }
    }

    fn pack(&mut self, value: &crate::ast::PackDecl) {
        for field in &value.fields {
            self.type_name(&field.ty);
            if let Some(default_value) = &field.default_value {
                self.expr(default_value);
            }
        }
        match &value.storage {
            crate::ast::PackStorage::Scalar(storage) => self.type_name(storage),
            crate::ast::PackStorage::ByteArray { type_name, element, .. } => {
                self.type_name(type_name);
                self.type_name(element);
            }
        }
    }

    fn serialize(&mut self, value: &crate::ast::SerializeDecl) {
        self.type_name(&value.source_type);
        for section in &value.sections {
            if let crate::ast::SerializeSection::Version { ty, .. } = section {
                self.type_name(ty);
            }
        }
    }

    fn perform(&mut self, value: &crate::ast::PerformDecl) {
        self.spelling(&value.role_name, value.span);
        self.type_name(&value.target);
        for verb in &value.methods {
            self.locals.clear();
            self.callable(&verb.generic_parameters, &verb.params, &verb.return_type);
            collect_block_bindings(&verb.body, &mut self.locals);
            self.block(&verb.body);
        }
    }

    fn type_name(&mut self, type_name: &TypeName) {
        self.spelling(&type_name.name, type_name.span);
        type_name.arguments.iter().for_each(|argument| self.type_name(argument));
    }

    /// Records every path inside `spelling`, which may carry a generic
    /// application such as `Holder[Inner, 4]`.
    fn spelling(&mut self, spelling: &str, span: SourceSpan) {
        for path in spelling_paths(spelling) {
            let head = path.split("::").next().unwrap_or(path);
            if !self.locals.contains(head) {
                self.references.push(Reference { path: path.to_owned(), span });
            }
        }
    }

    fn block(&mut self, block: &Block) {
        block.statements.iter().for_each(|statement| self.statement(statement));
    }

    fn statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::OwnerDecl { ty, initializer, span, .. } => {
                if let Some(ty) = ty {
                    self.spelling(ty, *span);
                }
                self.expr(initializer);
            }
            Stmt::Assignment { target, value, .. }
            | Stmt::CompoundAssignment { target, value, .. } => {
                self.place(target);
                self.expr(value);
            }
            Stmt::Expression { expression: value, .. }
            | Stmt::Return { value: Some(value), .. } => self.expr(value),
            Stmt::If { condition, then_branch, else_branch, .. } => {
                self.conditional(condition, then_branch, else_branch.as_ref())
            }
            Stmt::Loop(block) | Stmt::Block(block) => self.block(block),
            Stmt::ForRange { binding, start, end, body, .. } => {
                self.binding_type(binding);
                self.expr(start);
                self.expr(end);
                self.block(body);
            }
            Stmt::ForArray { binding, collection, body, .. } => {
                self.binding_type(binding);
                self.expr(collection);
                self.block(body);
            }
            Stmt::Return { value: None, .. }
            | Stmt::Drop { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }

    fn binding_type(&mut self, binding: &crate::ast::ForBinding) {
        if let Some(ty) = &binding.ty {
            self.spelling(ty, binding.span);
        }
    }

    fn place(&mut self, place: &Place) {
        match place {
            Place::Binding { .. } => {}
            Place::Field { object, .. } => self.place(object),
            Place::Index { target, index, .. } => {
                self.place(target);
                self.expr(index);
            }
        }
    }

    fn conditional(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&IfBranch>,
    ) {
        self.expr(condition);
        self.block(then_branch);
        match else_branch {
            Some(IfBranch::Block(block)) => self.block(block),
            Some(IfBranch::ElseIf(expression)) => self.expr(expression),
            None => {}
        }
    }

    fn expr(&mut self, expression: &Expr) {
        match expression {
            Expr::Identifier { name, span } => self.spelling(name, *span),
            Expr::Call { callee, generic_arguments, arguments, span, .. } => {
                self.spelling(callee, *span);
                generic_arguments.iter().for_each(|argument| self.type_name(argument));
                arguments.iter().for_each(|argument| self.expr(&argument.expression));
            }
            Expr::StructLit { name, type_arguments, fields, span, .. } => {
                self.spelling(name, *span);
                type_arguments.iter().for_each(|argument| self.type_name(argument));
                fields.iter().for_each(|field| self.expr(&field.value));
            }
            Expr::Cast { expression, target, .. } => {
                self.type_name(target);
                self.expr(expression);
            }
            Expr::MethodCall { receiver, arguments, .. } => {
                self.expr(receiver);
                arguments.iter().for_each(|argument| self.expr(&argument.expression));
            }
            Expr::Case { subject, branches, .. } => self.case(subject, branches),
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.conditional(condition, then_branch, else_branch.as_ref())
            }
            _ => self.nested_expr(expression),
        }
    }

    fn nested_expr(&mut self, expression: &Expr) {
        match expression {
            Expr::BufferLiteral { length: inner, .. }
            | Expr::Grouping { expression: inner, .. }
            | Expr::Unary { expression: inner, .. }
            | Expr::Borrow { expression: inner, .. }
            | Expr::Try { expression: inner, .. }
            | Expr::FieldAccess { object: inner, .. } => self.expr(inner),
            Expr::Binary { left, right, .. } | Expr::Index { target: left, index: right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            _ => {}
        }
    }

    fn case(&mut self, subject: &Expr, branches: &[crate::ast::CaseBranch]) {
        self.expr(subject);
        for branch in branches {
            if let Pattern::Variant { enum_name, span, .. } = &branch.pattern {
                self.spelling(enum_name, *span);
            }
            if let Some(guard) = &branch.guard {
                self.expr(guard);
            }
            match &branch.body {
                CaseBody::Expression(value) => self.expr(value),
                CaseBody::Block(block) => self.block(block),
            }
        }
    }
}

/// Splits a spelling into its path tokens: `Holder[Inner, 4]` yields
/// `Holder` and `Inner`; numeric arguments are not references.
pub(super) fn spelling_paths(spelling: &str) -> impl Iterator<Item = &str> {
    spelling
        .split(|character: char| {
            !(character.is_alphanumeric() || character == '_' || character == ':')
        })
        .map(|token| token.trim_matches(':'))
        .filter(|token| {
            token.chars().next().is_some_and(|first| first.is_alphabetic() || first == '_')
        })
}

fn collect_block_bindings(block: &Block, bindings: &mut HashSet<String>) {
    for statement in &block.statements {
        match statement {
            Stmt::OwnerDecl { name, initializer, .. } => {
                bindings.insert(name.clone());
                collect_expr_bindings(initializer, bindings);
            }
            Stmt::ForRange { binding, body, .. } | Stmt::ForArray { binding, body, .. } => {
                bindings.insert(binding.name.clone());
                collect_block_bindings(body, bindings);
            }
            Stmt::Loop(nested) | Stmt::Block(nested) => collect_block_bindings(nested, bindings),
            Stmt::If { condition, then_branch, else_branch, .. } => {
                collect_expr_bindings(condition, bindings);
                collect_block_bindings(then_branch, bindings);
                collect_branch_bindings(else_branch.as_ref(), bindings);
            }
            Stmt::Expression { expression, .. } | Stmt::Return { value: Some(expression), .. } => {
                collect_expr_bindings(expression, bindings)
            }
            Stmt::Assignment { value, .. } | Stmt::CompoundAssignment { value, .. } => {
                collect_expr_bindings(value, bindings)
            }
            Stmt::Return { value: None, .. }
            | Stmt::Drop { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }
}

fn collect_branch_bindings(branch: Option<&IfBranch>, bindings: &mut HashSet<String>) {
    match branch {
        Some(IfBranch::Block(block)) => collect_block_bindings(block, bindings),
        Some(IfBranch::ElseIf(expression)) => collect_expr_bindings(expression, bindings),
        None => {}
    }
}

/// Pattern bindings live inside `case` expressions, which may nest anywhere.
fn collect_expr_bindings(expression: &Expr, bindings: &mut HashSet<String>) {
    match expression {
        Expr::Case { subject, branches, .. } => {
            collect_expr_bindings(subject, bindings);
            for branch in branches {
                collect_pattern_bindings(&branch.pattern, bindings);
                if let Some(guard) = &branch.guard {
                    collect_expr_bindings(guard, bindings);
                }
                match &branch.body {
                    CaseBody::Expression(value) => collect_expr_bindings(value, bindings),
                    CaseBody::Block(block) => collect_block_bindings(block, bindings),
                }
            }
        }
        Expr::If { condition, then_branch, else_branch, .. } => {
            collect_expr_bindings(condition, bindings);
            collect_block_bindings(then_branch, bindings);
            collect_branch_bindings(else_branch.as_ref(), bindings);
        }
        Expr::Call { arguments, .. } | Expr::MethodCall { arguments, .. } => {
            if let Expr::MethodCall { receiver, .. } = expression {
                collect_expr_bindings(receiver, bindings);
            }
            for argument in arguments {
                collect_expr_bindings(&argument.expression, bindings);
            }
        }
        Expr::StructLit { fields, .. } => {
            fields.iter().for_each(|field| collect_expr_bindings(&field.value, bindings))
        }
        Expr::BufferLiteral { length: inner, .. }
        | Expr::Grouping { expression: inner, .. }
        | Expr::Unary { expression: inner, .. }
        | Expr::Borrow { expression: inner, .. }
        | Expr::Try { expression: inner, .. }
        | Expr::Cast { expression: inner, .. }
        | Expr::FieldAccess { object: inner, .. } => collect_expr_bindings(inner, bindings),
        Expr::Binary { left, right, .. } | Expr::Index { target: left, index: right, .. } => {
            collect_expr_bindings(left, bindings);
            collect_expr_bindings(right, bindings);
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => {}
    }
}

fn collect_pattern_bindings(pattern: &Pattern, bindings: &mut HashSet<String>) {
    let Pattern::Variant { payload, .. } = pattern else { return };
    match payload {
        VariantPayload::Unit => {}
        VariantPayload::Positional(items) => {
            bindings.extend(items.iter().map(|binding| binding.name.clone()))
        }
        VariantPayload::Named(items) => {
            bindings.extend(items.iter().map(|named| named.binding.name.clone()))
        }
    }
}
