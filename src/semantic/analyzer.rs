use std::collections::{HashMap, HashSet};

use crate::ast::{
    Block, BuiltinType, Expr, IntrinsicKind, Program, Role, Stmt, TopLevelDecl,
    lookup_builtin_type, lookup_call_intrinsic,
};

use super::errors::SemanticError;
use super::model::SemanticModel;

pub(super) struct ScopeFrame {
    pub(super) span: crate::lexer::SourceSpan,
    pub(super) bindings: HashMap<String, usize>,
    pub(super) borrow_ids: Vec<usize>,
    pub(super) declaration_indices: Vec<usize>,
}

pub(super) struct Analyzer {
    pub(super) model: SemanticModel,
    pub(super) scopes: Vec<ScopeFrame>,
    pub(super) next_borrow_id: usize,
    pub(super) active_borrow_ids: HashSet<usize>,
    pub(super) signatures: HashMap<String, super::calls::VerbSignature>,
    pub(super) loop_boundaries: Vec<usize>,
    pub(super) current_return_type: Option<BuiltinType>,
}

pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    Analyzer::new().analyze(program)
}

impl Analyzer {
    fn new() -> Self {
        Self {
            model: SemanticModel {
                bindings: Vec::new(),
                borrows: Vec::new(),
                cleanup_plans: Vec::new(),
                return_unwind_plans: Vec::new(),
                loop_unwind_plans: Vec::new(),
            },
            scopes: Vec::new(),
            next_borrow_id: 0,
            active_borrow_ids: HashSet::new(),
            signatures: HashMap::new(),
            loop_boundaries: Vec::new(),
            current_return_type: None,
        }
    }

    fn analyze(mut self, program: &Program) -> Result<SemanticModel, SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration;
            for parameter in &verb.params {
                self.validate_type_name(&parameter.ty.name, parameter.ty.span)?;
            }
            if let Some(return_type) = &verb.return_type {
                self.validate_type_name(&return_type.name, return_type.span)?;
            }
            if super::intrinsics::is_reserved_name(&verb.name) {
                return Err(SemanticError {
                    kind: super::errors::SemanticErrorKind::ReservedIntrinsicName {
                        name: verb.name.clone(),
                    },
                    span: verb.span,
                });
            }
            if self.signatures.contains_key(&verb.name) {
                return Err(SemanticError {
                    kind: super::errors::SemanticErrorKind::DuplicateVerbName {
                        name: verb.name.clone(),
                    },
                    span: verb.span,
                });
            }
            self.signatures.insert(verb.name.clone(), verb.signature());
        }
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration;
            self.current_return_type = verb
                .return_type
                .as_ref()
                .and_then(|type_name| lookup_builtin_type(&type_name.name));
            self.enter_scope(verb.body.span);
            for parameter in &verb.params {
                let ty = lookup_builtin_type(&parameter.ty.name);
                self.bind(parameter.role.clone(), parameter.name.clone(), ty, parameter.span)?;
            }
            self.visit_block(&verb.body)?;
            self.leave_scope();
        }
        self.current_return_type = None;
        Ok(self.model)
    }

    fn validate_type_name(
        &self,
        name: &str,
        span: crate::lexer::SourceSpan,
    ) -> Result<(), SemanticError> {
        if lookup_builtin_type(name).is_some() {
            return Ok(());
        }
        Err(SemanticError {
            kind: super::errors::SemanticErrorKind::UnknownType { name: name.to_owned() },
            span,
        })
    }

    fn visit_block(&mut self, block: &Block) -> Result<(), SemanticError> {
        for statement in &block.statements {
            self.visit_statement(statement)?;
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Stmt) -> Result<(), SemanticError> {
        match statement {
            Stmt::OwnerDecl { role, name, ty, initializer, span } => {
                let binding_type = self.resolve_binding_type(ty.as_deref(), initializer, *span)?;
                self.visit_expression(initializer)?;
                if *role == Role::Erg {
                    self.initialize_owner(initializer, *span)?;
                }
                if *role == Role::Abs {
                    self.register_borrow(initializer, *span)?;
                }
                self.bind(role.clone(), name.clone(), binding_type, *span)
            }
            Stmt::Assignment { name, value, span } => {
                let index = self.binding(name, *span)?;
                self.ensure_mutable(index, name, *span)?;
                self.visit_expression(value)
            }
            Stmt::Expression { expression, .. } => self.visit_expression(expression),
            Stmt::Return { value, span } => self.visit_return(value.as_ref(), *span),
            Stmt::Loop(block) => {
                self.enter_scope(block.span);
                self.loop_boundaries.push(self.scopes.len() - 1);
                self.visit_block(block)?;
                self.loop_boundaries.pop();
                self.leave_scope();
                Ok(())
            }
            Stmt::Break { span } => {
                self.plan_loop_unwind(super::cleanup::LoopExitKind::Break, "break", *span)
            }
            Stmt::Continue { span } => {
                self.plan_loop_unwind(super::cleanup::LoopExitKind::Continue, "continue", *span)
            }
            Stmt::Drop { name, span } => self.drop_binding(name, *span),
            Stmt::Block(block) => {
                self.enter_scope(block.span);
                self.visit_block(block)?;
                self.leave_scope();
                Ok(())
            }
        }
    }

    fn resolve_binding_type(
        &self,
        declared_type: Option<&str>,
        initializer: &Expr,
        span: crate::lexer::SourceSpan,
    ) -> Result<Option<BuiltinType>, SemanticError> {
        if let Some(name) = declared_type {
            self.validate_type_name(name, span)?;
            return Ok(lookup_builtin_type(name));
        }
        Ok(self.expression_type(initializer))
    }

    pub(super) fn expression_type(&self, expression: &Expr) -> Option<BuiltinType> {
        match expression {
            Expr::Integer { .. } => Some(BuiltinType::Int),
            Expr::Grouping { expression, .. }
            | Expr::Borrow { expression, .. }
            | Expr::Unary { expression, .. } => self.expression_type(expression),
            Expr::Binary { .. } => Some(BuiltinType::Int),
            Expr::Identifier { name, span } => {
                self.binding(name, *span).ok().and_then(|index| self.model.bindings[index].ty)
            }
            Expr::Call { callee, .. } => match lookup_call_intrinsic(callee) {
                Some(IntrinsicKind::Allocate) => Some(BuiltinType::Buffer),
                Some(IntrinsicKind::Append) => Some(BuiltinType::Int),
                Some(IntrinsicKind::Drop) => None,
                None => self.signatures.get(callee).and_then(|signature| signature.return_type),
            },
            Expr::StringLiteral { .. } => None,
        }
    }

    pub(super) fn visit_expression(&mut self, expression: &Expr) -> Result<(), SemanticError> {
        match expression {
            Expr::Identifier { name, span } => {
                let index = self.binding(name, *span)?;
                self.ensure_readable(index, name, *span)
            }
            Expr::Binary { left, right, .. } => {
                self.visit_expression(left)?;
                self.visit_expression(right)
            }
            Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
                self.visit_expression(expression)
            }
            Expr::Borrow { expression, .. } => self.visit_expression(expression),
            Expr::Call { callee, arguments, span } => self.visit_call(callee, arguments, *span),
            Expr::Integer { .. } | Expr::StringLiteral { .. } => Ok(()),
        }
    }
}
