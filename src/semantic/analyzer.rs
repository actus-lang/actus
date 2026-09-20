use std::collections::{HashMap, HashSet};

use crate::ast::{Block, Expr, Program, Role, Stmt, TopLevelDecl};

use super::errors::SemanticError;
use super::model::SemanticModel;

pub(super) struct ScopeFrame {
    pub(super) bindings: HashMap<String, usize>,
    pub(super) borrow_ids: Vec<usize>,
}

pub(super) struct Analyzer {
    pub(super) model: SemanticModel,
    pub(super) scopes: Vec<ScopeFrame>,
    pub(super) next_borrow_id: usize,
    pub(super) active_borrow_ids: HashSet<usize>,
    pub(super) signatures: HashMap<String, super::calls::VerbSignature>,
}

pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    Analyzer::new().analyze(program)
}

impl Analyzer {
    fn new() -> Self {
        Self {
            model: SemanticModel { bindings: Vec::new(), borrows: Vec::new() },
            scopes: Vec::new(),
            next_borrow_id: 0,
            active_borrow_ids: HashSet::new(),
            signatures: HashMap::new(),
        }
    }

    fn analyze(mut self, program: &Program) -> Result<SemanticModel, SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration;
            self.signatures.insert(verb.name.clone(), verb.signature());
        }
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration;
            self.enter_scope();
            for parameter in &verb.params {
                self.bind(parameter.role.clone(), parameter.name.clone(), parameter.span)?;
            }
            self.visit_block(&verb.body)?;
            self.leave_scope();
        }
        Ok(self.model)
    }

    fn visit_block(&mut self, block: &Block) -> Result<(), SemanticError> {
        for statement in &block.statements {
            self.visit_statement(statement)?;
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Stmt) -> Result<(), SemanticError> {
        match statement {
            Stmt::OwnerDecl { role, name, initializer, span, .. } => {
                self.visit_expression(initializer)?;
                if *role == Role::Abs {
                    self.register_borrow(initializer, *span)?;
                }
                self.bind(role.clone(), name.clone(), *span)
            }
            Stmt::Assignment { name, value, span } => {
                let index = self.binding(name, *span)?;
                self.ensure_mutable(index, name, *span)?;
                self.visit_expression(value)
            }
            Stmt::Expression { expression, .. } => self.visit_expression(expression),
            Stmt::Return { value, .. } => {
                value.as_ref().map_or(Ok(()), |value| self.visit_expression(value))
            }
            Stmt::Drop { name, span } => self.drop_binding(name, *span),
            Stmt::Block(block) => {
                self.enter_scope();
                self.visit_block(block)?;
                self.leave_scope();
                Ok(())
            }
        }
    }

    pub(super) fn visit_expression(&mut self, expression: &Expr) -> Result<(), SemanticError> {
        match expression {
            Expr::Identifier { name, span } => {
                let index = self.binding(name, *span)?;
                self.ensure_readable(index, name, *span)
            }
            Expr::Borrow { expression, .. } => self.visit_expression(expression),
            Expr::Call { callee, arguments, span } => self.visit_call(callee, arguments, *span),
            Expr::Integer { .. } | Expr::StringLiteral { .. } => Ok(()),
        }
    }
}
