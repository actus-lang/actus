use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ast::{
    Block, BuiltinType, EnumDef, Expr, Program, Role, Stmt, StructDef, TopLevelDecl,
    lookup_builtin_type,
};

use super::errors::{SemanticError, SemanticErrorKind};
use super::model::SemanticModel;

pub(super) struct ScopeFrame {
    pub(super) span: crate::lexer::SourceSpan,
    pub(super) bindings: HashMap<String, usize>,
    pub(super) borrow_ids: Vec<usize>,
    pub(super) declaration_indices: Vec<usize>,
    pub(super) payload_cleanup: Vec<(usize, String, String, String)>,
}

pub(super) struct Analyzer {
    pub(super) model: SemanticModel,
    pub(super) scopes: Vec<ScopeFrame>,
    pub(super) next_borrow_id: usize,
    pub(super) active_borrow_ids: HashSet<usize>,
    pub(super) signatures: HashMap<String, super::calls::VerbSignature>,
    pub(super) loop_boundaries: Vec<usize>,
    pub(super) current_return_type: Option<BuiltinType>,
    pub(super) struct_types: HashMap<String, StructDef>,
    pub(super) enum_types: HashMap<String, EnumDef>,
    pub(super) binding_struct_types: HashMap<usize, String>,
    pub(super) binding_struct_type_applications: HashMap<usize, crate::ast::TypeName>,
    pub(super) binding_enum_types: HashMap<usize, String>,
    pub(super) generic_scopes: Vec<HashSet<String>>,
    pub(super) generic_instances: BTreeMap<String, super::model::GenericInstance>,
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
                generic_instances: Vec::new(),
            },
            scopes: Vec::new(),
            next_borrow_id: 0,
            active_borrow_ids: HashSet::new(),
            signatures: HashMap::new(),
            loop_boundaries: Vec::new(),
            current_return_type: None,
            struct_types: HashMap::new(),
            enum_types: HashMap::new(),
            binding_struct_types: HashMap::new(),
            binding_struct_type_applications: HashMap::new(),
            binding_enum_types: HashMap::new(),
            generic_scopes: Vec::new(),
            generic_instances: BTreeMap::new(),
        }
    }

    fn analyze(mut self, program: &Program) -> Result<SemanticModel, SemanticError> {
        self.register_enums(program)?;
        self.register_structs(program)?;
        self.validate_recursive_types()?;
        self.register_declarations(program)?;
        self.validate_method_declarations(program)?;
        self.analyze_verbs(program)?;
        self.model.generic_instances = self.generic_instances.into_values().collect();
        self.current_return_type = None;
        Ok(self.model)
    }

    fn register_declarations(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let (name, params, return_type, span, signature) = match declaration {
                TopLevelDecl::Verb(verb) => {
                    (&verb.name, &verb.params, &verb.return_type, verb.span, verb.signature())
                }
                TopLevelDecl::ExternalVerb(verb) => {
                    (&verb.name, &verb.params, &verb.return_type, verb.span, verb.signature())
                }
                TopLevelDecl::Struct(_) | TopLevelDecl::Enum(_) => continue,
            };
            let generic_parameters = match declaration {
                TopLevelDecl::Verb(verb) => &verb.generic_parameters,
                TopLevelDecl::ExternalVerb(verb) => &verb.generic_parameters,
                TopLevelDecl::Struct(_) | TopLevelDecl::Enum(_) => unreachable!(),
            };
            self.with_generic_scope(generic_parameters, |analyzer| {
                for parameter in params {
                    analyzer.validate_type_reference(&parameter.ty)?;
                }
                if let Some(return_type) = return_type {
                    analyzer.validate_type_reference(return_type)?;
                }
                Ok(())
            })?;
            if super::intrinsics::is_reserved_name(name) {
                return Err(SemanticError {
                    kind: super::errors::SemanticErrorKind::ReservedIntrinsicName {
                        name: name.clone(),
                    },
                    span,
                });
            }
            if self.signatures.contains_key(name) {
                return Err(SemanticError {
                    kind: super::errors::SemanticErrorKind::DuplicateVerbName {
                        name: name.clone(),
                    },
                    span,
                });
            }
            self.signatures.insert(name.clone(), signature);
        }
        Ok(())
    }

    fn analyze_verbs(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration else { continue };
            self.current_return_type = verb
                .return_type
                .as_ref()
                .and_then(|type_name| lookup_builtin_type(&type_name.name));
            self.enter_scope(verb.body.span);
            for parameter in &verb.params {
                let ty = lookup_builtin_type(&parameter.ty.name);
                self.bind(parameter.role.clone(), parameter.name.clone(), ty, parameter.span)?;
                self.record_struct_binding(&parameter.name, &parameter.ty, parameter.span)?;
                self.record_enum_binding(&parameter.name, &parameter.ty.name, parameter.span)?;
            }
            self.visit_block(&verb.body)?;
            if self.current_return_type.is_some() && !block_guarantees_return(&verb.body) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MissingReturnValue,
                    span: verb.body.span,
                });
            }
            self.leave_scope();
        }
        Ok(())
    }

    pub(super) fn visit_block(&mut self, block: &Block) -> Result<(), SemanticError> {
        for statement in &block.statements {
            self.visit_statement(statement)?;
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Stmt) -> Result<(), SemanticError> {
        match statement {
            Stmt::OwnerDecl { role, name, ty, initializer, span } => {
                let binding_type = self.resolve_binding_type(ty.as_deref(), initializer, *span)?;
                self.validate_declared_initializer(name, ty.as_deref(), initializer, *span)?;
                self.visit_expression(initializer)?;
                if *role == Role::Erg {
                    self.initialize_owner(initializer, *span)?;
                }
                if *role == Role::Abs {
                    self.register_borrow(initializer, *span)?;
                }
                self.bind(role.clone(), name.clone(), binding_type, *span)?;
                self.record_initializer_struct_type(name, initializer, *span)?;
                self.record_initializer_enum_type(name, initializer, *span)
            }
            Stmt::Assignment { name, value, span } => {
                let index = self.binding(name, *span)?;
                self.ensure_mutable(index, name, *span)?;
                self.validate_binding_assignment(index, name, value, *span)?;
                self.visit_expression(value)
            }
            Stmt::FieldAssignment { object, field, value, span } => {
                self.validate_field_assignment(object, field, value, *span)
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
            Expr::MethodCall { receiver, method, arguments, span } => {
                self.visit_method_call(receiver, method, arguments, *span)
            }
            Expr::StructLit { name, type_arguments, fields, span } => {
                self.validate_struct_literal(name, type_arguments, fields, *span)
            }
            Expr::FieldAccess { object, field, span } => {
                if self.enum_receiver_name(object).is_some() {
                    self.validate_enum_unit_variant(object, field, *span)
                } else {
                    self.visit_expression(object)?;
                    self.validate_field_access(object, field, *span)
                }
            }
            Expr::Case { mode, subject, branches, span } => {
                self.visit_expression(subject)?;
                self.validate_case_patterns(*mode, subject, branches, *span)
            }
            Expr::Integer { .. } | Expr::FloatLiteral { .. } | Expr::StringLiteral { .. } => Ok(()),
        }
    }
}

fn block_guarantees_return(block: &Block) -> bool {
    block.statements.iter().any(statement_guarantees_return)
}

fn statement_guarantees_return(statement: &Stmt) -> bool {
    match statement {
        Stmt::Return { .. } => true,
        Stmt::Loop(block) => loop_guarantees_return(block),
        Stmt::Block(block) => block_guarantees_return(block),
        _ => false,
    }
}

fn loop_guarantees_return(block: &Block) -> bool {
    block_guarantees_return(block) && !contains_loop_exit(block)
}

fn contains_loop_exit(block: &Block) -> bool {
    block.statements.iter().any(|statement| match statement {
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::Block(nested) => contains_loop_exit(nested),
        Stmt::Loop(_) => false,
        _ => false,
    })
}
