use std::collections::HashSet;

use crate::ast::{ConstantDecl, Expr, Program, TopLevelDecl};

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

impl Analyzer {
    pub(super) fn register_constants(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Constant(constant) = declaration else { continue };
            self.validate_type_reference(&constant.ty)?;
            if self.constants.contains_key(&constant.name) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateConstantName { name: constant.name.clone() },
                    span: constant.span,
                });
            }
            self.constants.insert(constant.name.clone(), constant.ty.clone());
            self.constant_initializers.insert(constant.name.clone(), constant.initializer.clone());
        }
        for declaration in &program.declarations {
            let TopLevelDecl::Constant(constant) = declaration else { continue };
            self.validate_constant(constant)?;
        }
        for name in self.constants.keys().cloned().collect::<Vec<_>>() {
            if self.constant_has_cycle(&name, &mut Vec::new()) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::ConstantCycle { name: name.clone() },
                    span: program
                        .declarations
                        .iter()
                        .find_map(|declaration| match declaration {
                            TopLevelDecl::Constant(constant) if constant.name == name => {
                                Some(constant.span)
                            }
                            _ => None,
                        })
                        .unwrap_or(crate::lexer::SourceSpan::new(0, 0)),
                });
            }
        }
        Ok(())
    }

    fn validate_constant(&mut self, constant: &ConstantDecl) -> Result<(), SemanticError> {
        if !is_compile_time_expression(&constant.initializer, &self.constants) {
            return Err(SemanticError {
                kind: SemanticErrorKind::ConstantRuntimeDependency { name: constant.name.clone() },
                span: constant.span,
            });
        }
        self.visit_expression(&constant.initializer)?;
        self.validate_expected_literal(&constant.initializer, &constant.ty)?;
        let literal_initializer = matches!(
            constant.initializer,
            crate::ast::Expr::Integer { .. } | crate::ast::Expr::FloatLiteral { .. }
        );
        if !literal_initializer
            && let Some(found) = self.expression_type_name(&constant.initializer)
            && found != super::canonical_type_name(&constant.ty)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::BindingTypeMismatch {
                    binding: constant.name.clone(),
                    expected: super::canonical_type_name(&constant.ty),
                    found,
                },
                span: constant.span,
            });
        }
        Ok(())
    }

    fn constant_has_cycle(&self, name: &str, path: &mut Vec<String>) -> bool {
        if path.iter().any(|entry| entry == name) {
            return true;
        }
        let Some(initializer) = self.constant_initializers.get(name) else { return false };
        path.push(name.to_owned());
        let dependencies = constant_dependencies(initializer, &self.constants);
        let cycle = dependencies.iter().any(|dependency| self.constant_has_cycle(dependency, path));
        path.pop();
        cycle
    }
}

fn is_compile_time_expression(
    expression: &Expr,
    constants: &std::collections::HashMap<String, crate::ast::TypeName>,
) -> bool {
    match expression {
        Expr::Integer { .. } | Expr::FloatLiteral { .. } => true,
        Expr::Identifier { name, .. } => constants.contains_key(name),
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. } => is_compile_time_expression(expression, constants),
        Expr::Binary { left, right, .. } => {
            is_compile_time_expression(left, constants)
                && is_compile_time_expression(right, constants)
        }
        Expr::BufferLiteral { .. }
        | Expr::StringLiteral { .. }
        | Expr::Borrow { .. }
        | Expr::Try { .. }
        | Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. }
        | Expr::Index { .. }
        | Expr::Case { .. }
        | Expr::If { .. } => false,
    }
}

fn constant_dependencies(
    expression: &Expr,
    constants: &std::collections::HashMap<String, crate::ast::TypeName>,
) -> HashSet<String> {
    let mut dependencies = HashSet::new();
    collect_dependencies(expression, constants, &mut dependencies);
    dependencies
}

fn collect_dependencies(
    expression: &Expr,
    constants: &std::collections::HashMap<String, crate::ast::TypeName>,
    dependencies: &mut HashSet<String>,
) {
    match expression {
        Expr::Identifier { name, .. } => {
            if constants.contains_key(name) {
                dependencies.insert(name.clone());
            }
        }
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. } => collect_dependencies(expression, constants, dependencies),
        Expr::Binary { left, right, .. } => {
            collect_dependencies(left, constants, dependencies);
            collect_dependencies(right, constants, dependencies);
        }
        Expr::Call { arguments, .. } | Expr::MethodCall { arguments, .. } => {
            for argument in arguments {
                collect_dependencies(&argument.expression, constants, dependencies);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_dependencies(&field.value, constants, dependencies);
            }
        }
        Expr::FieldAccess { object, .. } => collect_dependencies(object, constants, dependencies),
        Expr::Index { target, index, .. } => {
            collect_dependencies(target, constants, dependencies);
            collect_dependencies(index, constants, dependencies);
        }
        Expr::Case { subject, branches, .. } => {
            collect_dependencies(subject, constants, dependencies);
            for branch in branches {
                if let Some(guard) = &branch.guard {
                    collect_dependencies(guard, constants, dependencies);
                }
                match &branch.body {
                    crate::ast::CaseBody::Expression(value) => {
                        collect_dependencies(value, constants, dependencies)
                    }
                    crate::ast::CaseBody::Block(_) => {}
                }
            }
        }
        Expr::If { condition, .. } => collect_dependencies(condition, constants, dependencies),
        Expr::Integer { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => {}
    }
}
