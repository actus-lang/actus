use crate::semantic::{CleanupAction, SemanticModel};

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

pub fn lower_cleanup_plans(model: &SemanticModel) -> Vec<NativeCleanupPlan> {
    model
        .cleanup_plans
        .iter()
        .map(|scope| NativeCleanupPlan {
            depth: scope.depth,
            instructions: scope.actions.iter().map(lower_action).collect(),
        })
        .collect()
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
