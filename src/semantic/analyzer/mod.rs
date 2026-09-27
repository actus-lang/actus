use std::collections::{HashMap, HashSet};

use crate::ast::{
    Block, BuiltinType, EnumDef, Expr, Program, ReturnAccess, RoleDecl, Stmt, StructDef,
    TopLevelDecl, lookup_builtin_type,
};
use crate::lexer::SourceSpan;

use super::errors::{SemanticError, SemanticErrorKind};
use super::model::Origin;
use super::model::SemanticModel;

mod expressions;
mod statements;

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
    pub(super) next_loan_id: usize,
    pub(super) active_borrow_ids: HashSet<usize>,
    pub(super) signatures: HashMap<String, super::calls::VerbSignature>,
    pub(super) loop_boundaries: Vec<usize>,
    pub(super) current_return_type: Option<BuiltinType>,
    pub(super) current_return_type_name: Option<crate::ast::TypeName>,
    pub(super) current_return_is_builtin_result: bool,
    pub(super) current_return_access: Option<ReturnAccess>,
    pub(super) current_abs_origins: HashMap<String, usize>,
    pub(super) binding_origins: HashMap<usize, Origin>,
    pub(super) struct_types: HashMap<String, StructDef>,
    pub(super) pack_types: HashMap<String, crate::ast::PackDecl>,
    pub(super) enum_types: HashMap<String, EnumDef>,
    pub(super) role_types: HashMap<String, RoleDecl>,
    pub(super) performances: HashSet<(String, String)>,
    pub(super) performance_methods: HashMap<(String, String), super::calls::VerbSignature>,
    pub(super) performance_roles: HashMap<(String, String), String>,
    pub(super) reachable_performances: HashSet<super::model::ReachablePerformance>,
    pub(super) drop_types: HashSet<String>,
    pub(super) binding_struct_types: HashMap<usize, String>,
    pub(super) binding_struct_type_applications: HashMap<usize, crate::ast::TypeName>,
    pub(super) binding_type_names: HashMap<usize, crate::ast::TypeName>,
    pub(super) binding_enum_types: HashMap<usize, String>,
    pub(super) binding_enum_type_applications: HashMap<usize, crate::ast::TypeName>,
    pub(super) binding_dynamic_roles: HashMap<usize, String>,
    pub(super) binding_arena_provenance: HashMap<usize, usize>,
    pub(super) binding_scope_depth: HashMap<usize, usize>,
    pub(super) arena_scope_depth: HashMap<usize, usize>,
    pub(super) field_arena_provenance: HashMap<(usize, String), usize>,
    pub(super) expression_arena_provenance: HashMap<(usize, usize), usize>,
    pub(super) next_arena_id: usize,
    pub(super) generic_scopes: Vec<HashSet<String>>,
    pub(super) generic_bounds: HashMap<String, Vec<String>>,
    pub(super) generic_instances: super::generic_cache::GenericInstanceCache,
    pub(super) expected_expression_type: Option<crate::ast::TypeName>,
    pub(super) inferred_expression_types: HashMap<(usize, usize), crate::ast::TypeName>,
    pub(super) type_registry: super::types::TypeRegistry,
}

pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    Analyzer::new().analyze(program)
}

impl Analyzer {
    fn require_buffer_length(&self, expression: &Expr) -> Result<(), SemanticError> {
        if self.expression_type(expression) == Some(BuiltinType::Int) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidIntrinsicArgument {
                callee: "Buffer".to_owned(),
                parameter: "length".to_owned(),
            },
            span: expression_span(expression),
        })
    }

    fn new() -> Self {
        Self {
            model: SemanticModel {
                bindings: Vec::new(),
                borrows: Vec::new(),
                exclusive_loans: Vec::new(),
                expression_origins: Vec::new(),
                cleanup_plans: Vec::new(),
                return_unwind_plans: Vec::new(),
                loop_unwind_plans: Vec::new(),
                generic_instances: Vec::new(),
                reachable_performances: Vec::new(),
                dynamic_roles: Vec::new(),
                drop_types: Vec::new(),
                binding_type_names: HashMap::new(),
                arena_provenance: HashMap::new(),
            },
            scopes: Vec::new(),
            next_borrow_id: 0,
            next_loan_id: 0,
            active_borrow_ids: HashSet::new(),
            signatures: HashMap::new(),
            loop_boundaries: Vec::new(),
            current_return_type: None,
            current_return_type_name: None,
            current_return_is_builtin_result: false,
            current_return_access: None,
            current_abs_origins: HashMap::new(),
            binding_origins: HashMap::new(),
            struct_types: HashMap::new(),
            pack_types: HashMap::new(),
            enum_types: HashMap::new(),
            role_types: HashMap::new(),
            performances: HashSet::new(),
            performance_methods: HashMap::new(),
            performance_roles: HashMap::new(),
            reachable_performances: HashSet::new(),
            drop_types: HashSet::new(),
            binding_struct_types: HashMap::new(),
            binding_struct_type_applications: HashMap::new(),
            binding_type_names: HashMap::new(),
            binding_enum_types: HashMap::new(),
            binding_enum_type_applications: HashMap::new(),
            binding_dynamic_roles: HashMap::new(),
            binding_arena_provenance: HashMap::new(),
            binding_scope_depth: HashMap::new(),
            arena_scope_depth: HashMap::new(),
            field_arena_provenance: HashMap::new(),
            expression_arena_provenance: HashMap::new(),
            next_arena_id: 0,
            generic_scopes: Vec::new(),
            generic_bounds: HashMap::new(),
            generic_instances: super::generic_cache::GenericInstanceCache::for_current_toolchain(),
            expected_expression_type: None,
            inferred_expression_types: HashMap::new(),
            type_registry: super::types::TypeRegistry::new(),
        }
    }

    fn analyze(mut self, program: &Program) -> Result<SemanticModel, SemanticError> {
        self.register_enums(program)?;
        self.register_roles(program)?;
        self.register_structs(program)?;
        self.validate_pack_declarations(program)?;
        self.validate_role_declarations()?;
        self.validate_performances(program)?;
        self.validate_recursive_types()?;
        self.register_declarations(program)?;
        self.collect_dynamic_roles(program);
        self.validate_method_declarations(program)?;
        self.analyze_verbs(program)?;
        self.model.generic_instances = self.generic_instances.into_instances();
        self.model.reachable_performances = self.reachable_performances.into_iter().collect();
        self.model.reachable_performances.sort_by(|left, right| {
            (&left.target_type, &left.role_name, &left.method_name).cmp(&(
                &right.target_type,
                &right.role_name,
                &right.method_name,
            ))
        });
        self.model.drop_types = self.drop_types.into_iter().collect();
        self.model.drop_types.sort();
        self.model.binding_type_names = self.binding_type_names.clone();
        self.model.arena_provenance = self.binding_arena_provenance.clone();
        self.current_return_type = None;
        self.current_return_type_name = None;
        self.current_return_access = None;
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
                TopLevelDecl::Struct(_)
                | TopLevelDecl::Pack(_)
                | TopLevelDecl::Enum(_)
                | TopLevelDecl::Role(_)
                | TopLevelDecl::Perform(_) => continue,
                TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_) => continue,
            };
            let generic_parameters = match declaration {
                TopLevelDecl::Verb(verb) => &verb.generic_parameters,
                TopLevelDecl::ExternalVerb(verb) => &verb.generic_parameters,
                TopLevelDecl::Struct(_)
                | TopLevelDecl::Pack(_)
                | TopLevelDecl::Enum(_)
                | TopLevelDecl::Role(_)
                | TopLevelDecl::Perform(_) => unreachable!(),
                TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_) => unreachable!(),
            };
            self.with_generic_scope(generic_parameters, |analyzer| {
                for parameter in params {
                    analyzer.validate_dynamic_parameter(parameter)?;
                    if parameter.dispatch == crate::ast::DispatchMode::Static {
                        analyzer.validate_type_reference(&parameter.ty)?;
                    }
                }
                if let Some(return_type) = return_type {
                    analyzer.validate_type_reference(&return_type.ty)?;
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
            if let TopLevelDecl::Verb(verb) = declaration {
                let parameters = verb.generic_parameters.clone();
                self.with_generic_scope(&parameters, |analyzer| analyzer.analyze_verb_body(verb))?;
            }
        }
        for declaration in &program.declarations {
            if let TopLevelDecl::Perform(perform) = declaration {
                for method in &perform.methods {
                    let parameters = method.generic_parameters.clone();
                    self.with_generic_scope(&parameters, |analyzer| {
                        analyzer.analyze_verb_body(method)
                    })?;
                }
            }
        }
        Ok(())
    }

    fn analyze_verb_body(&mut self, verb: &crate::ast::VerbDecl) -> Result<(), SemanticError> {
        self.current_return_access =
            verb.return_type.as_ref().map(|return_type| return_type.access);
        self.current_return_type = verb
            .return_type
            .as_ref()
            .and_then(|return_type| lookup_builtin_type(&return_type.ty.name));
        self.current_return_type_name =
            verb.return_type.as_ref().map(|return_type| return_type.ty.clone());
        self.current_return_is_builtin_result =
            verb.return_type.as_ref().is_some_and(|return_type| {
                return_type.ty.name == "Result"
                    && self
                        .enum_types
                        .get("Result")
                        .is_some_and(|definition| definition.span.start == 0)
            });
        self.initialize_origin_parameter_map(&verb.params);
        self.enter_scope(verb.body.span);
        for parameter in &verb.params {
            let ty = lookup_builtin_type(&parameter.ty.name);
            self.bind(parameter.role.clone(), parameter.name.clone(), ty, parameter.span)?;
            let index = self.binding(&parameter.name, parameter.span)?;
            self.binding_type_names.insert(index, parameter.ty.clone());
            if parameter.dispatch == crate::ast::DispatchMode::Dynamic {
                let index = self.binding(&parameter.name, parameter.span)?;
                self.binding_dynamic_roles.insert(index, parameter.ty.name.clone());
            }
            self.record_struct_binding(&parameter.name, &parameter.ty, parameter.span)?;
            self.record_enum_binding(&parameter.name, &parameter.ty, parameter.span)?;
            if let Ok(index) = self.binding(&parameter.name, parameter.span)
                && let Some(parameter_index) = self.current_abs_origins.get(&parameter.name)
            {
                self.binding_origins
                    .insert(index, Origin::AbsParameter { parameter_index: *parameter_index });
            }
        }
        self.visit_block(&verb.body)?;
        let requires_return_value =
            self.current_return_type.is_some() || self.current_return_is_builtin_result;
        if requires_return_value && !block_guarantees_return(&verb.body) {
            return Err(SemanticError {
                kind: SemanticErrorKind::MissingReturnValue,
                span: verb.body.span,
            });
        }
        self.leave_scope();
        Ok(())
    }
}

fn try_type_mismatch(
    span: crate::lexer::SourceSpan,
    expected: &crate::ast::TypeName,
    found: &str,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::TypeMismatch {
            callee: "?".to_owned(),
            parameter: "Result".to_owned(),
            expected: canonical_type_name(expected),
            found: found.to_owned(),
        },
        span,
    }
}

pub(super) fn canonical_type_name(type_name: &crate::ast::TypeName) -> String {
    let role = type_name
        .reference_role
        .as_ref()
        .map(|role| match role {
            crate::ast::Role::Abs => "abs ",
            crate::ast::Role::Ins => "ins ",
            crate::ast::Role::Erg => "erg ",
            crate::ast::Role::Dat => "dat ",
        })
        .unwrap_or("");
    if type_name.arguments.is_empty() {
        return format!("{role}{}", type_name.name);
    }
    format!(
        "{role}{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

fn block_guarantees_return(block: &Block) -> bool {
    block.statements.iter().any(statement_guarantees_return)
}

fn is_origin_return_expression(expression: &Expr) -> bool {
    match expression {
        Expr::Borrow { expression, .. } | Expr::Grouping { expression, .. } => {
            is_origin_return_expression(expression)
        }
        Expr::Call { .. } | Expr::MethodCall { .. } => true,
        _ => false,
    }
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

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Case { span, .. } => *span,
    }
}
