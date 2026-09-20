use std::collections::{HashMap, HashSet};

use crate::ast::{Block, Expr, Program, Role, Stmt, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::model::{Binding, BindingState, BorrowRecord, SemanticModel};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticErrorKind {
    DuplicateBinding { name: String },
    ShadowedBinding { name: String },
    UndeclaredIdentifier { name: String },
    InvalidBorrowTarget { name: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: SourceSpan,
}

pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    Analyzer::new().analyze(program)
}

struct ScopeFrame {
    bindings: HashMap<String, usize>,
    borrow_ids: Vec<usize>,
}

struct Analyzer {
    model: SemanticModel,
    scopes: Vec<ScopeFrame>,
    next_borrow_id: usize,
    active_borrow_ids: HashSet<usize>,
}

impl Analyzer {
    fn new() -> Self {
        Self {
            model: SemanticModel { bindings: Vec::new(), borrows: Vec::new() },
            scopes: Vec::new(),
            next_borrow_id: 0,
            active_borrow_ids: HashSet::new(),
        }
    }

    fn analyze(mut self, program: &Program) -> Result<SemanticModel, SemanticError> {
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
                self.binding(name, *span)?;
                self.visit_expression(value)
            }
            Stmt::Expression { expression, .. } => self.visit_expression(expression),
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.visit_expression(value)?;
                }
                Ok(())
            }
            Stmt::Drop { name, span } => {
                self.binding(name, *span)?;
                Ok(())
            }
            Stmt::Block(block) => {
                self.enter_scope();
                self.visit_block(block)?;
                self.leave_scope();
                Ok(())
            }
        }
    }

    fn visit_expression(&mut self, expression: &Expr) -> Result<(), SemanticError> {
        match expression {
            Expr::Identifier { name, span } => {
                self.binding(name, *span)?;
                Ok(())
            }
            Expr::Borrow { expression, .. } => self.visit_expression(expression),
            Expr::Call { arguments, .. } => {
                for argument in arguments {
                    self.visit_expression(&argument.expression)?;
                }
                Ok(())
            }
            Expr::Integer { .. } | Expr::StringLiteral { .. } => Ok(()),
        }
    }

    fn register_borrow(
        &mut self,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Expr::Borrow { expression, .. } = initializer else {
            return Ok(());
        };
        let Expr::Identifier { name, span: owner_span } = expression.as_ref() else {
            return Ok(());
        };
        let owner_index = self.binding(name, *owner_span)?;
        if self.model.bindings[owner_index].role != Role::Erg {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidBorrowTarget { name: name.clone() },
                span,
            });
        }
        let borrow_id = self.next_borrow_id;
        self.next_borrow_id += 1;
        self.model.borrows.push(BorrowRecord {
            id: borrow_id,
            owner: name.clone(),
            scope_depth: self.scopes.len(),
            origin_span: span,
        });
        self.active_borrow_ids.insert(borrow_id);
        self.scopes.last_mut().expect("a verb always has a scope").borrow_ids.push(borrow_id);
        match &mut self.model.bindings[owner_index].state {
            BindingState::Active => {
                self.model.bindings[owner_index].state =
                    BindingState::Frozen { borrow_ids: vec![borrow_id] }
            }
            BindingState::Frozen { borrow_ids } => borrow_ids.push(borrow_id),
            BindingState::Moved | BindingState::Dropped => {}
        }
        Ok(())
    }

    fn bind(&mut self, role: Role, name: String, span: SourceSpan) -> Result<(), SemanticError> {
        if self.scopes.last().expect("binding requires a scope").bindings.contains_key(&name) {
            return Err(SemanticError { kind: SemanticErrorKind::DuplicateBinding { name }, span });
        }
        if self.scopes.iter().rev().skip(1).any(|scope| scope.bindings.contains_key(&name)) {
            return Err(SemanticError { kind: SemanticErrorKind::ShadowedBinding { name }, span });
        }
        let index = self.model.bindings.len();
        self.model.bindings.push(Binding {
            name: name.clone(),
            role,
            span,
            state: BindingState::Active,
        });
        self.scopes.last_mut().expect("binding requires a scope").bindings.insert(name, index);
        Ok(())
    }

    fn binding(&self, name: &str, span: SourceSpan) -> Result<usize, SemanticError> {
        self.scopes.iter().rev().find_map(|scope| scope.bindings.get(name).copied()).ok_or_else(
            || SemanticError {
                kind: SemanticErrorKind::UndeclaredIdentifier { name: name.to_owned() },
                span,
            },
        )
    }

    fn enter_scope(&mut self) {
        self.scopes.push(ScopeFrame { bindings: HashMap::new(), borrow_ids: Vec::new() });
    }

    fn leave_scope(&mut self) {
        let frame = self.scopes.pop().expect("scope stack cannot be empty");
        for borrow_id in frame.borrow_ids {
            let Some(record) = self.model.borrows.iter().find(|record| record.id == borrow_id)
            else {
                continue;
            };
            let owner = record.owner.clone();
            self.active_borrow_ids.remove(&borrow_id);
            let owner_is_still_borrowed =
                self.model.borrows.iter().any(|other| {
                    other.owner == owner && self.active_borrow_ids.contains(&other.id)
                });
            if !owner_is_still_borrowed
                && let Some(index) =
                    self.model.bindings.iter().position(|binding| binding.name == owner)
            {
                self.model.bindings[index].state = BindingState::Active;
            }
        }
    }
}
