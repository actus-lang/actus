//! Rewrites spellings qualified by a binding of the unit's own modules
//! (`runtime::count` after `import pkg::runtime` inside unit `pkg`) to the
//! unit-local name (`count`).
//!
//! Every declaration of a unit has a unit-unique name (ADR-0110), so the
//! qualifier carries no identity; the scope check has already decided on
//! the original sources whether the spelling is in scope. The implementation
//! program then names the unit's own definition directly, private
//! declarations included, instead of an imported external symbol.

use std::collections::BTreeSet;

use crate::ast::{
    Block, CaseBody, EnumPayload, Expr, GenericParam, IfBranch, Param, Pattern, Place, ReturnType,
    Stmt, TopLevelDecl, TypeName,
};

use super::tree::ModuleTree;

pub(super) struct Qualifiers<'a> {
    pub(super) tree: &'a ModuleTree,
    pub(super) bindings: &'a BTreeSet<String>,
}

impl Qualifiers<'_> {
    pub(super) fn declaration(&self, declaration: &mut TopLevelDecl) {
        match declaration {
            TopLevelDecl::Constant(value) => {
                self.type_name(&mut value.ty);
                self.expr(&mut value.initializer);
            }
            TopLevelDecl::Verb(value) => {
                self.callable(
                    &mut value.generic_parameters,
                    &mut value.params,
                    &mut value.return_type,
                );
                self.block(&mut value.body);
            }
            TopLevelDecl::ExternalVerb(value) => self.callable(
                &mut value.generic_parameters,
                &mut value.params,
                &mut value.return_type,
            ),
            TopLevelDecl::Struct(value) => {
                self.generics(&mut value.generic_parameters);
                value.fields.iter_mut().for_each(|field| self.type_name(&mut field.ty));
            }
            TopLevelDecl::Enum(value) => self.enumeration(value),
            TopLevelDecl::Pack(value) => self.pack(value),
            TopLevelDecl::Serialize(value) => self.serialize(value),
            TopLevelDecl::Role(value) => {
                for method in &mut value.methods {
                    self.callable(&mut Vec::new(), &mut method.params, &mut method.return_type);
                }
            }
            TopLevelDecl::Perform(value) => self.perform(value),
            TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_) => {}
        }
    }

    fn enumeration(&self, value: &mut crate::ast::EnumDef) {
        self.generics(&mut value.generic_parameters);
        for variant in &mut value.variants {
            match &mut variant.payload {
                EnumPayload::Tuple(types) => types.iter_mut().for_each(|ty| self.type_name(ty)),
                EnumPayload::Struct(fields) => {
                    fields.iter_mut().for_each(|field| self.type_name(&mut field.ty))
                }
                EnumPayload::Unit => {}
            }
        }
    }

    fn serialize(&self, value: &mut crate::ast::SerializeDecl) {
        self.type_name(&mut value.source_type);
        for section in &mut value.sections {
            if let crate::ast::SerializeSection::Version { ty, .. } = section {
                self.type_name(ty);
            }
        }
    }

    fn perform(&self, value: &mut crate::ast::PerformDecl) {
        value.role_name = self.spelling(&value.role_name);
        self.type_name(&mut value.target);
        for verb in &mut value.methods {
            self.callable(&mut verb.generic_parameters, &mut verb.params, &mut verb.return_type);
            self.block(&mut verb.body);
        }
    }

    fn callable(
        &self,
        generics: &mut [GenericParam],
        params: &mut [Param],
        return_type: &mut Option<ReturnType>,
    ) {
        self.generics(generics);
        params.iter_mut().for_each(|param| self.type_name(&mut param.ty));
        if let Some(return_type) = return_type {
            self.type_name(&mut return_type.ty);
        }
    }

    fn generics(&self, generics: &mut [GenericParam]) {
        for parameter in generics {
            parameter
                .bound
                .iter_mut()
                .chain(&mut parameter.bounds)
                .for_each(|bound| self.type_name(bound));
            if let crate::ast::GenericParamKind::Const { domain } = &mut parameter.kind {
                self.type_name(domain);
            }
        }
    }

    fn pack(&self, value: &mut crate::ast::PackDecl) {
        for field in &mut value.fields {
            self.type_name(&mut field.ty);
            if let Some(default_value) = &mut field.default_value {
                self.expr(default_value);
            }
        }
        match &mut value.storage {
            crate::ast::PackStorage::Scalar(storage) => self.type_name(storage),
            crate::ast::PackStorage::ByteArray { type_name, element, .. } => {
                self.type_name(type_name);
                self.type_name(element);
            }
        }
    }

    fn type_name(&self, type_name: &mut TypeName) {
        type_name.name = self.spelling(&type_name.name);
        type_name.arguments.iter_mut().for_each(|argument| self.type_name(argument));
    }

    /// Strips an internal binding from every path of `spelling` whose leaf
    /// the unit declares; generic applications such as
    /// `Holder[runtime::Inner, 4]` are rewritten token by token.
    fn spelling(&self, spelling: &str) -> String {
        let mut rewritten = String::with_capacity(spelling.len());
        let mut token = String::new();
        for character in spelling.chars().chain(std::iter::once('\0')) {
            if character.is_alphanumeric() || character == '_' || character == ':' {
                token.push(character);
                continue;
            }
            rewritten.push_str(&self.path(&token));
            token.clear();
            if character != '\0' {
                rewritten.push(character);
            }
        }
        rewritten
    }

    fn path(&self, path: &str) -> String {
        match path.split_once("::") {
            Some((head, leaf))
                if self.bindings.contains(head)
                    && !leaf.contains("::")
                    && self.tree.owner(leaf).is_some() =>
            {
                leaf.to_owned()
            }
            _ => path.to_owned(),
        }
    }

    fn block(&self, block: &mut Block) {
        block.statements.iter_mut().for_each(|statement| self.statement(statement));
    }

    fn statement(&self, statement: &mut Stmt) {
        match statement {
            Stmt::OwnerDecl { ty, initializer, .. } => {
                if let Some(ty) = ty {
                    *ty = self.spelling(ty);
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
                self.conditional(condition, then_branch, else_branch.as_mut())
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

    fn binding_type(&self, binding: &mut crate::ast::ForBinding) {
        if let Some(ty) = &mut binding.ty {
            *ty = self.spelling(ty);
        }
    }

    fn place(&self, place: &mut Place) {
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
        &self,
        condition: &mut Expr,
        then_branch: &mut Block,
        else_branch: Option<&mut IfBranch>,
    ) {
        self.expr(condition);
        self.block(then_branch);
        match else_branch {
            Some(IfBranch::Block(block)) => self.block(block),
            Some(IfBranch::ElseIf(expression)) => self.expr(expression),
            None => {}
        }
    }

    fn expr(&self, expression: &mut Expr) {
        match expression {
            Expr::Identifier { name, .. } => *name = self.spelling(name),
            Expr::Call { callee, generic_arguments, arguments, .. } => {
                *callee = self.spelling(callee);
                generic_arguments.iter_mut().for_each(|argument| self.type_name(argument));
                arguments.iter_mut().for_each(|argument| self.expr(&mut argument.expression));
            }
            Expr::StructLit { name, type_arguments, fields, .. } => {
                *name = self.spelling(name);
                type_arguments.iter_mut().for_each(|argument| self.type_name(argument));
                fields.iter_mut().for_each(|field| self.expr(&mut field.value));
            }
            Expr::Cast { expression, target, .. } => {
                self.type_name(target);
                self.expr(expression);
            }
            Expr::MethodCall { receiver, arguments, .. } => {
                self.expr(receiver);
                arguments.iter_mut().for_each(|argument| self.expr(&mut argument.expression));
            }
            Expr::Case { subject, branches, .. } => self.case(subject, branches),
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.conditional(condition, then_branch, else_branch.as_mut())
            }
            _ => self.nested_expr(expression),
        }
    }

    fn nested_expr(&self, expression: &mut Expr) {
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

    fn case(&self, subject: &mut Expr, branches: &mut [crate::ast::CaseBranch]) {
        self.expr(subject);
        for branch in branches {
            if let Pattern::Variant { enum_name, .. } = &mut branch.pattern {
                *enum_name = self.spelling(enum_name);
            }
            if let Some(guard) = &mut branch.guard {
                self.expr(guard);
            }
            match &mut branch.body {
                CaseBody::Expression(value) => self.expr(value),
                CaseBody::Block(block) => self.block(block),
            }
        }
    }
}
