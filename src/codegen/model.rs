use crate::semantic::{CleanupAction, LoopExitKind, SemanticModel};

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
    pub scopes: Vec<NativeCleanupPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeLoopUnwindPlan {
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
        .map(|plan| NativeUnwindPlan { scopes: plan.scopes.iter().map(lower_scope).collect() })
        .collect()
}

pub fn lower_loop_unwind_plans(model: &SemanticModel) -> Vec<NativeLoopUnwindPlan> {
    model
        .loop_unwind_plans
        .iter()
        .map(|plan| NativeLoopUnwindPlan {
            kind: plan.kind.clone(),
            scopes: plan.scopes.iter().map(lower_scope).collect(),
        })
        .collect()
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
