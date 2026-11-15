use crate::ast::{Expr, GenericParam, TopLevelDecl};

use super::DeclarationRegistry;

/// Where a type spelling is resolved: the module, and the generic parameters
/// of the declaration being annotated together with their owner's key.
#[derive(Clone, Copy)]
pub(super) struct Scope<'a> {
    pub(super) module_path: &'a str,
    pub(super) generic_owner: &'a str,
    pub(super) generic_parameters: &'a [GenericParam],
}

impl<'a> Scope<'a> {
    pub(super) fn module(module_path: &'a str) -> Self {
        Self { module_path, generic_owner: "", generic_parameters: &[] }
    }

    fn generic(self, generic_owner: &'a str, generic_parameters: &'a [GenericParam]) -> Self {
        Self { module_path: self.module_path, generic_owner, generic_parameters }
    }
}

impl DeclarationRegistry {
    pub(crate) fn annotate_program_headers(
        &self,
        module_path: &str,
        program: &mut crate::ast::Program,
    ) {
        for declaration in &mut program.declarations {
            self.annotate_declaration(Scope::module(module_path), declaration);
        }
    }

    fn annotate_declaration(&self, scope: Scope<'_>, declaration: &mut TopLevelDecl) {
        match declaration {
            TopLevelDecl::Constant(value) => {
                self.resolve_scoped_type_name(scope, &mut value.ty);
                self.annotate_expr(scope, &mut value.initializer);
            }
            TopLevelDecl::Verb(value) => {
                value.identity =
                    value.identity.or_else(|| self.verb_id(scope.module_path, &value.name));
                let owner = self.generic_owner(scope, &value.name, &value.generic_parameters);
                let scope = scope.generic(&owner, &value.generic_parameters);
                annotate_callable(self, scope, &mut value.params, &mut value.return_type);
                self.annotate_block(scope, &mut value.body);
            }
            TopLevelDecl::ExternalVerb(value) => {
                // An imported alias resolves its spelling (`ns::verb`) to the
                // identity of its source verb.
                value.identity =
                    value.identity.or_else(|| self.verb_id(scope.module_path, &value.name));
                let owner = self.generic_owner(scope, &value.name, &value.generic_parameters);
                let scope = scope.generic(&owner, &value.generic_parameters);
                annotate_callable(self, scope, &mut value.params, &mut value.return_type)
            }
            TopLevelDecl::Struct(value) => {
                let owner = self.generic_owner(scope, &value.name, &value.generic_parameters);
                let scope = scope.generic(&owner, &value.generic_parameters);
                for field in &mut value.fields {
                    self.resolve_scoped_type_name(scope, &mut field.ty);
                }
            }
            TopLevelDecl::Enum(value) => {
                let owner = self.generic_owner(scope, &value.name, &value.generic_parameters);
                self.annotate_enum(
                    scope.generic(&owner, &value.generic_parameters),
                    &mut value.variants,
                );
            }
            TopLevelDecl::Pack(value) => self.annotate_pack(scope, value),
            TopLevelDecl::Serialize(value) => {
                self.resolve_scoped_type_name(scope, &mut value.source_type);
                for section in &mut value.sections {
                    if let crate::ast::SerializeSection::Version { ty, .. } = section {
                        self.resolve_scoped_type_name(scope, ty);
                    }
                }
            }
            TopLevelDecl::Perform(value) => self.annotate_perform(scope, value),
            TopLevelDecl::Role(value) => self.annotate_role(scope, value),
            TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_) => {}
        }
    }

    fn annotate_enum(&self, scope: Scope<'_>, variants: &mut [crate::ast::EnumVariant]) {
        for variant in variants {
            match &mut variant.payload {
                crate::ast::EnumPayload::Tuple(fields) => {
                    for field in fields {
                        self.resolve_scoped_type_name(scope, field);
                    }
                }
                crate::ast::EnumPayload::Struct(fields) => {
                    for field in fields {
                        self.resolve_scoped_type_name(scope, &mut field.ty);
                    }
                }
                crate::ast::EnumPayload::Unit => {}
            }
        }
    }

    fn annotate_pack(&self, scope: Scope<'_>, value: &mut crate::ast::PackDecl) {
        for field in &mut value.fields {
            self.resolve_scoped_type_name(scope, &mut field.ty);
            if let Some(default_value) = &mut field.default_value {
                self.annotate_expr(scope, default_value);
            }
        }
        match &mut value.storage {
            crate::ast::PackStorage::Scalar(value) => self.resolve_scoped_type_name(scope, value),
            crate::ast::PackStorage::ByteArray { type_name, element, .. } => {
                self.resolve_scoped_type_name(scope, type_name);
                self.resolve_scoped_type_name(scope, element);
            }
        }
    }

    fn annotate_perform(&self, scope: Scope<'_>, value: &mut crate::ast::PerformDecl) {
        self.resolve_scoped_type_name(scope, &mut value.target);
        for verb in &mut value.methods {
            let owner = if verb.generic_parameters.is_empty() {
                String::new()
            } else {
                format!("{}::{}::{}", value.target.canonical_key(), value.role_name, verb.name)
            };
            let scope = scope.generic(&owner, &verb.generic_parameters);
            annotate_callable(self, scope, &mut verb.params, &mut verb.return_type);
            self.annotate_block(scope, &mut verb.body);
        }
    }

    fn annotate_role(&self, scope: Scope<'_>, value: &mut crate::ast::RoleDecl) {
        for method in &mut value.methods {
            annotate_callable(self, scope, &mut method.params, &mut method.return_type);
        }
    }

    fn annotate_block(&self, scope: Scope<'_>, block: &mut crate::ast::Block) {
        for statement in &mut block.statements {
            self.annotate_stmt(scope, statement);
        }
    }

    fn annotate_stmt(&self, scope: Scope<'_>, statement: &mut crate::ast::Stmt) {
        use crate::ast::Stmt;

        match statement {
            Stmt::OwnerDecl { ty, resolved_type, initializer, span, .. } => {
                if resolved_type.is_none()
                    && let Some(spelling) = ty
                    && let Some(mut type_name) =
                        crate::semantic::parse_type_name_key(spelling, *span)
                {
                    self.resolve_scoped_type_name(scope, &mut type_name);
                    *resolved_type = Some(type_name);
                }
                self.annotate_expr(scope, initializer);
            }
            Stmt::Assignment { target, value, .. }
            | Stmt::CompoundAssignment { target, value, .. } => {
                self.annotate_place(scope, target);
                self.annotate_expr(scope, value);
            }
            Stmt::Expression { expression, .. } => self.annotate_expr(scope, expression),
            Stmt::If { condition, then_branch, else_branch, .. } => {
                self.annotate_conditional_stmt(scope, condition, then_branch, else_branch)
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.annotate_expr(scope, value);
                }
            }
            Stmt::Loop(block) | Stmt::Block(block) => self.annotate_block(scope, block),
            Stmt::ForRange { start, end, body, .. } => {
                self.annotate_for_range(scope, start, end, body)
            }
            Stmt::ForArray { collection, body, .. } => {
                self.annotate_for_array(scope, collection, body)
            }
            Stmt::Drop { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn annotate_conditional_stmt(
        &self,
        scope: Scope<'_>,
        condition: &mut crate::ast::Expr,
        then_branch: &mut crate::ast::Block,
        else_branch: &mut Option<crate::ast::IfBranch>,
    ) {
        self.annotate_expr(scope, condition);
        self.annotate_block(scope, then_branch);
        if let Some(branch) = else_branch {
            self.annotate_if_branch(scope, branch);
        }
    }

    fn annotate_for_range(
        &self,
        scope: Scope<'_>,
        start: &mut crate::ast::Expr,
        end: &mut crate::ast::Expr,
        body: &mut crate::ast::Block,
    ) {
        self.annotate_expr(scope, start);
        self.annotate_expr(scope, end);
        self.annotate_block(scope, body);
    }

    fn annotate_for_array(
        &self,
        scope: Scope<'_>,
        collection: &mut crate::ast::Expr,
        body: &mut crate::ast::Block,
    ) {
        self.annotate_expr(scope, collection);
        self.annotate_block(scope, body);
    }

    fn annotate_if_branch(&self, scope: Scope<'_>, branch: &mut crate::ast::IfBranch) {
        match branch {
            crate::ast::IfBranch::Block(block) => self.annotate_block(scope, block),
            crate::ast::IfBranch::ElseIf(expression) => self.annotate_expr(scope, expression),
        }
    }

    fn annotate_place(&self, scope: Scope<'_>, place: &mut crate::ast::Place) {
        if let crate::ast::Place::Index { index, .. } = place {
            self.annotate_expr(scope, index);
        }
        if let crate::ast::Place::Field { object, .. } = place {
            self.annotate_place(scope, object);
        }
    }

    fn annotate_expr(&self, scope: Scope<'_>, expression: &mut crate::ast::Expr) {
        use crate::ast::Expr;

        match expression {
            Expr::BufferLiteral { length, .. }
            | Expr::Grouping { expression: length, .. }
            | Expr::Unary { expression: length, .. }
            | Expr::Borrow { expression: length, .. }
            | Expr::Try { expression: length, .. } => self.annotate_expr(scope, length),
            Expr::Cast { expression, target, .. } => {
                self.resolve_scoped_type_name(scope, target);
                self.annotate_expr(scope, expression);
            }
            Expr::Binary { left, right, .. } => {
                self.annotate_expr(scope, left);
                self.annotate_expr(scope, right);
            }
            Expr::Call { .. } => self.annotate_call(scope, expression),
            Expr::MethodCall { .. } => self.annotate_method_call(scope, expression),
            Expr::StructLit { .. } => self.annotate_struct_literal(scope, expression),
            Expr::FieldAccess { .. } => self.annotate_field_access(scope, expression),
            Expr::Index { target, index, .. } => {
                self.annotate_expr(scope, target);
                self.annotate_expr(scope, index);
            }
            Expr::Case { subject, branches, .. } => self.annotate_case(scope, subject, branches),
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.annotate_if_expr(scope, condition, then_branch, else_branch)
            }
            Expr::Identifier { .. }
            | Expr::Integer { .. }
            | Expr::BoolLiteral { .. }
            | Expr::FloatLiteral { .. }
            | Expr::StringLiteral { .. } => {}
        }
    }

    fn annotate_call(&self, scope: Scope<'_>, expression: &mut crate::ast::Expr) {
        let crate::ast::Expr::Call { callee, resolved_type, generic_arguments, arguments, span } =
            expression
        else {
            unreachable!()
        };
        for type_argument in generic_arguments {
            self.resolve_scoped_type_name(scope, type_argument);
        }
        if resolved_type.is_none()
            && callee_base(callee).is_some_and(|base| self.resolves_type(scope.module_path, base))
            && let Some(mut type_name) = crate::semantic::parse_type_name_key(callee, *span)
        {
            self.resolve_scoped_type_name(scope, &mut type_name);
            *resolved_type = Some(type_name);
        }
        for argument in arguments {
            self.annotate_expr(scope, &mut argument.expression);
        }
    }

    fn annotate_method_call(&self, scope: Scope<'_>, expression: &mut crate::ast::Expr) {
        let crate::ast::Expr::MethodCall { receiver, arguments, resolved_type, span, .. } =
            expression
        else {
            unreachable!()
        };
        if resolved_type.is_none()
            && let Some(mut type_name) = receiver_type_name(receiver, *span)
        {
            self.resolve_scoped_type_name(scope, &mut type_name);
            if type_name.resolved.is_some() {
                *resolved_type = Some(type_name);
            }
        }
        self.annotate_expr(scope, receiver);
        for argument in arguments {
            self.annotate_expr(scope, &mut argument.expression);
        }
    }

    fn annotate_struct_literal(&self, scope: Scope<'_>, expression: &mut crate::ast::Expr) {
        let Expr::StructLit { name, type_arguments, resolved_type, fields, span } = expression
        else {
            unreachable!()
        };
        for type_argument in type_arguments.iter_mut() {
            self.resolve_scoped_type_name(scope, type_argument);
        }
        if resolved_type.is_none()
            && let Some(identity) = self.resolve(scope.module_path, name)
        {
            *resolved_type = Some(crate::ast::TypeName::resolved(
                identity,
                name.clone(),
                type_arguments.clone(),
                None,
                *span,
            ));
        }
        for field in fields {
            self.annotate_expr(scope, &mut field.value);
        }
    }

    fn annotate_field_access(&self, scope: Scope<'_>, expression: &mut crate::ast::Expr) {
        let Expr::FieldAccess { object, resolved_type, span, .. } = expression else {
            unreachable!()
        };
        if resolved_type.is_none()
            && let Some(mut type_name) = receiver_type_name(object, *span)
        {
            self.resolve_scoped_type_name(scope, &mut type_name);
            if type_name.resolved.is_some() {
                *resolved_type = Some(type_name);
            }
        }
        self.annotate_expr(scope, object);
    }

    fn annotate_case(
        &self,
        scope: Scope<'_>,
        subject: &mut crate::ast::Expr,
        branches: &mut [crate::ast::CaseBranch],
    ) {
        use crate::ast::CaseBody;
        self.annotate_expr(scope, subject);
        for branch in branches {
            if let Some(guard) = &mut branch.guard {
                self.annotate_expr(scope, guard);
            }
            match &mut branch.body {
                CaseBody::Expression(value) => self.annotate_expr(scope, value),
                CaseBody::Block(block) => self.annotate_block(scope, block),
            }
        }
    }

    fn annotate_if_expr(
        &self,
        scope: Scope<'_>,
        condition: &mut crate::ast::Expr,
        then_branch: &mut crate::ast::Block,
        else_branch: &mut Option<crate::ast::IfBranch>,
    ) {
        use crate::ast::IfBranch;
        self.annotate_expr(scope, condition);
        self.annotate_block(scope, then_branch);
        if let Some(branch) = else_branch {
            match branch {
                IfBranch::Block(block) => self.annotate_block(scope, block),
                IfBranch::ElseIf(expression) => self.annotate_expr(scope, expression),
            }
        }
    }
}

fn callee_base(callee: &str) -> Option<&str> {
    callee.split_once('[').map(|(base, _)| base)
}

fn receiver_type_name(
    receiver: &Expr,
    span: crate::lexer::SourceSpan,
) -> Option<crate::ast::TypeName> {
    match receiver {
        Expr::Identifier { name, .. } => crate::ast::parse_type_name_with_mode(
            name,
            span,
            crate::ast::TypeNameParseMode::Permissive,
        ),
        Expr::Index { target, index, .. } => {
            let Expr::Identifier { name, .. } = target.as_ref() else { return None };
            let Expr::Identifier { name: argument, .. } = index.as_ref() else { return None };
            let mut type_name = crate::ast::parse_type_name_with_mode(
                name,
                span,
                crate::ast::TypeNameParseMode::Permissive,
            )?;
            let argument = crate::ast::parse_type_name_with_mode(
                argument,
                span,
                crate::ast::TypeNameParseMode::Permissive,
            )?;
            type_name.arguments.push(argument);
            Some(type_name)
        }
        _ => None,
    }
}

fn annotate_callable(
    registry: &DeclarationRegistry,
    scope: Scope<'_>,
    params: &mut [crate::ast::Param],
    return_type: &mut Option<crate::ast::ReturnType>,
) {
    for param in params {
        registry.resolve_scoped_type_name(scope, &mut param.ty);
    }
    if let Some(return_type) = return_type {
        registry.resolve_scoped_type_name(scope, &mut return_type.ty);
    }
}

impl super::declarations::DeclarationRegistry {
    /// The canonical identity of the verb `spelling` names: only a declared
    /// verb has one, an unresolved or ambiguous spelling has none.
    fn verb_id(&self, module_path: &str, spelling: &str) -> Option<crate::ast::DeclarationId> {
        self.resolve_id(module_path, spelling)
            .filter(|id| matches!(id.identity(), crate::ast::DeclarationIdentity::Declaration(_)))
    }
}
