use crate::lexer::SourceSpan;
use crate::semantic::{CleanupAction, LoopExitKind, SemanticModel};
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeInstruction {
    EndBorrow { borrow_id: usize },
    DropBinding { binding_index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCleanupPlan {
    pub depth: usize,
    pub instructions: Vec<NativeInstruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeUnwindPlan {
    pub span: SourceSpan,
    pub scopes: Vec<NativeCleanupPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeLoopUnwindPlan {
    pub span: SourceSpan,
    pub kind: LoopExitKind,
    pub scopes: Vec<NativeCleanupPlan>,
}

pub fn lower_cleanup_plans(model: &SemanticModel) -> Vec<NativeCleanupPlan> {
    model.cleanup_plans.iter().map(lower_scope).collect()
}

pub fn lower_return_unwind_plans(model: &SemanticModel) -> Vec<NativeUnwindPlan> {
    model
        .return_unwind_plans
        .iter()
        .map(|plan| NativeUnwindPlan {
            span: plan.span,
            scopes: plan.scopes.iter().map(lower_scope).collect(),
        })
        .collect()
}

pub fn lower_loop_unwind_plans(model: &SemanticModel) -> Vec<NativeLoopUnwindPlan> {
    model
        .loop_unwind_plans
        .iter()
        .map(|plan| NativeLoopUnwindPlan {
            span: plan.span,
            kind: plan.kind.clone(),
            scopes: plan.scopes.iter().map(lower_scope).collect(),
        })
        .collect()
}

pub(super) fn validate_cleanup_plans(model: &SemanticModel) -> Result<(), String> {
    for plan in &model.cleanup_plans {
        validate_scope(plan, model)?;
    }
    for plan in &model.return_unwind_plans {
        for scope in &plan.scopes {
            validate_scope(scope, model)?;
        }
    }
    for plan in &model.loop_unwind_plans {
        for scope in &plan.scopes {
            validate_scope(scope, model)?;
        }
    }
    Ok(())
}

fn validate_scope(
    scope: &crate::semantic::ScopeCleanup,
    model: &SemanticModel,
) -> Result<(), String> {
    let mut bindings = HashSet::new();
    let mut borrows = HashSet::new();
    for action in &scope.actions {
        match action {
            CleanupAction::EndBorrow { borrow_id } => {
                if !borrows.insert(*borrow_id) {
                    return Err(format!("cleanup repeats borrow `{borrow_id}`"));
                }
                if !model.borrows.iter().any(|borrow| borrow.id == *borrow_id) {
                    return Err(format!("cleanup references unknown borrow `{borrow_id}`"));
                }
            }
            CleanupAction::DropBinding { binding_index }
                if *binding_index >= model.bindings.len() =>
            {
                return Err(format!("cleanup references invalid binding `{binding_index}`"));
            }
            CleanupAction::DropBinding { binding_index } => {
                if !bindings.insert(*binding_index) {
                    return Err(format!("cleanup repeats binding `{binding_index}`"));
                }
            }
        }
    }
    Ok(())
}

fn lower_scope(scope: &crate::semantic::ScopeCleanup) -> NativeCleanupPlan {
    NativeCleanupPlan {
        depth: scope.depth,
        instructions: scope.actions.iter().map(lower_action).collect(),
    }
}

fn lower_action(action: &CleanupAction) -> NativeInstruction {
    match action {
        CleanupAction::EndBorrow { borrow_id } => {
            NativeInstruction::EndBorrow { borrow_id: *borrow_id }
        }
        CleanupAction::DropBinding { binding_index } => {
            NativeInstruction::DropBinding { binding_index: *binding_index }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_cleanup_plans;
    use crate::semantic::{CleanupAction, ScopeCleanup, SemanticModel};

    fn model_with(action: CleanupAction) -> SemanticModel {
        SemanticModel {
            bindings: Vec::new(),
            borrows: Vec::new(),
            cleanup_plans: vec![ScopeCleanup { depth: 1, actions: vec![action] }],
            return_unwind_plans: Vec::new(),
            loop_unwind_plans: Vec::new(),
        }
    }

    #[test]
    fn rejects_cleanup_actions_with_unknown_references() {
        let binding = model_with(CleanupAction::DropBinding { binding_index: 0 });
        let borrow = model_with(CleanupAction::EndBorrow { borrow_id: 0 });

        assert!(validate_cleanup_plans(&binding).is_err());
        assert!(validate_cleanup_plans(&borrow).is_err());
    }

    #[test]
    fn rejects_duplicate_cleanup_actions_in_one_scope() {
        let model = SemanticModel {
            bindings: vec![crate::semantic::Binding {
                name: "value".to_owned(),
                role: crate::ast::Role::Erg,
                span: crate::lexer::SourceSpan::new(0, 1),
                state: crate::semantic::BindingState::Active,
            }],
            borrows: Vec::new(),
            cleanup_plans: vec![ScopeCleanup {
                depth: 1,
                actions: vec![
                    CleanupAction::DropBinding { binding_index: 0 },
                    CleanupAction::DropBinding { binding_index: 0 },
                ],
            }],
            return_unwind_plans: Vec::new(),
            loop_unwind_plans: Vec::new(),
        };

        assert!(validate_cleanup_plans(&model).is_err());
    }
}
