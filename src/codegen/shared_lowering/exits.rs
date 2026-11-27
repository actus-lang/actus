//! The exit lists of the semantic model with every owner resolved into the
//! bridge calls that release it (ADR-0125): one schedule per program, one set
//! of exits per verb body.
//!
//! The analyzer decides which owners each exit releases; this module only
//! looks up how each one is released, from the layouts of the program. Code
//! generation then runs the list of an exit where the exit is lowered and
//! derives nothing itself.

use crate::codegen::model::LoweredCleanupInstruction;
use crate::codegen::{BackendCleanupResolvedDescriptor, BackendContractError};
use crate::lexer::SourceSpan;
use crate::semantic::{CleanupAction, LoopExitKind, ScopeCleanup, SemanticModel};

use super::layout::ProgramLayouts;

/// What the lowering of a verb body needs to release owners: the cleanup
/// plans recorded on each function as metadata, and the exit lists resolved
/// into the steps that run.
pub(crate) type CleanupInputs<'a> = (&'a [crate::codegen::BackendCleanupPlan], &'a CleanupSchedule);

/// A test of the variant tag an enum carries at `offset` from the owner's
/// root: the step runs only when the tag of `size` bytes equals
/// `discriminant`, which selects the variant that owns the payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CleanupGuard {
    pub(crate) offset: u32,
    pub(crate) size: u8,
    pub(crate) discriminant: u32,
}

/// One bridge call of a release, addressed relative to the owner's value,
/// with the variant tests that must all hold for it to run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CleanupStep {
    pub(crate) descriptor: BackendCleanupResolvedDescriptor,
    pub(crate) guards: Vec<CleanupGuard>,
}

/// The release of one owner: the binding by its index (and the name it is
/// declared with, under which the lowering holds its current value), the
/// action the instructions record, and the bridge calls in their order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OwnerRelease {
    pub(crate) binding_index: usize,
    pub(crate) name: String,
    pub(crate) action: LoweredCleanupInstruction,
    pub(crate) steps: Vec<CleanupStep>,
}

/// The owners one scope releases, in reverse declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScopeRelease {
    pub(crate) span: SourceSpan,
    pub(crate) owners: Vec<OwnerRelease>,
}

/// Every exit of one verb body with what it releases. A scope end releases
/// one scope; a `return` and a `break` or `continue` release the scopes they
/// leave, innermost first.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct VerbExits {
    scope_ends: Vec<ScopeRelease>,
    returns: Vec<(SourceSpan, Vec<ScopeRelease>)>,
    loops: Vec<(SourceSpan, LoopExitKind, Vec<ScopeRelease>)>,
    drops: Vec<(SourceSpan, OwnerRelease)>,
    replacements: Vec<(SourceSpan, OwnerRelease)>,
}

impl VerbExits {
    /// What the end of the scope at `span` releases on its fallthrough path.
    pub(crate) fn scope_end(&self, span: SourceSpan) -> Option<&ScopeRelease> {
        self.scope_ends.iter().find(|scope| scope.span == span)
    }

    /// What the `return` (or `?` failure exit) at `span` releases.
    pub(crate) fn return_exit(&self, span: SourceSpan) -> Option<&[ScopeRelease]> {
        self.returns.iter().find(|(at, _)| *at == span).map(|(_, scopes)| scopes.as_slice())
    }

    /// What the `break` or `continue` at `span` releases.
    pub(crate) fn loop_exit(
        &self,
        span: SourceSpan,
        kind: &LoopExitKind,
    ) -> Option<&[ScopeRelease]> {
        self.loops
            .iter()
            .find(|(at, exit, _)| *at == span && exit == kind)
            .map(|(_, _, scopes)| scopes.as_slice())
    }

    /// What the explicit `drop` at `span` releases.
    pub(crate) fn explicit_drop(&self, span: SourceSpan) -> Option<&OwnerRelease> {
        self.drops.iter().find(|(at, _)| *at == span).map(|(_, owner)| owner)
    }

    /// What the assignment at `span` releases before it stores a new value.
    pub(crate) fn replacement(&self, span: SourceSpan) -> Option<&OwnerRelease> {
        self.replacements.iter().find(|(at, _)| *at == span).map(|(_, owner)| owner)
    }
}

/// The exits of every verb body of a program, by the body's name and span.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct CleanupSchedule {
    verbs: Vec<(String, SourceSpan, VerbExits)>,
}

impl CleanupSchedule {
    /// Resolves every exit list of `model` with the layouts of the program.
    pub(crate) fn build(
        model: &SemanticModel,
        layouts: &ProgramLayouts<'_>,
    ) -> Result<Self, BackendContractError> {
        let resolver = Resolver { model, layouts };
        let mut verbs = Vec::with_capacity(model.verb_cleanups.len());
        for record in &model.verb_cleanups {
            let exits = VerbExits {
                scope_ends: model.cleanup_plans[record.scopes.clone()]
                    .iter()
                    .map(|scope| resolver.scope(scope))
                    .collect::<Result<_, _>>()?,
                returns: model.return_unwind_plans[record.returns.clone()]
                    .iter()
                    .map(|plan| Ok((plan.span, resolver.scopes(&plan.scopes)?)))
                    .collect::<Result<_, BackendContractError>>()?,
                loops: model.loop_unwind_plans[record.loops.clone()]
                    .iter()
                    .map(|plan| Ok((plan.span, plan.kind.clone(), resolver.scopes(&plan.scopes)?)))
                    .collect::<Result<_, BackendContractError>>()?,
                drops: resolver.drops(&model.explicit_drops[record.drops.clone()])?,
                replacements: resolver
                    .drops(&model.replacement_drops[record.replacements.clone()])?,
            };
            verbs.push((record.name.clone(), record.span, exits));
        }
        Ok(Self { verbs })
    }

    /// The exits of the verb body `name` declared at `span`.
    pub(crate) fn verb(&self, name: &str, span: SourceSpan) -> Option<&VerbExits> {
        self.verbs
            .iter()
            .find(|(candidate, at, _)| candidate == name && *at == span)
            .map(|(_, _, exits)| exits)
    }
}

struct Resolver<'a, 'program> {
    model: &'a SemanticModel,
    layouts: &'a ProgramLayouts<'program>,
}

impl Resolver<'_, '_> {
    fn scopes(&self, scopes: &[ScopeCleanup]) -> Result<Vec<ScopeRelease>, BackendContractError> {
        scopes.iter().map(|scope| self.scope(scope)).collect()
    }

    fn scope(&self, scope: &ScopeCleanup) -> Result<ScopeRelease, BackendContractError> {
        let mut owners = Vec::new();
        for action in &scope.actions {
            if let Some(owner) = self.action(action)? {
                owners.push(owner);
            }
        }
        Ok(ScopeRelease { span: scope.span, owners })
    }

    fn drops(
        &self,
        drops: &[crate::semantic::ExplicitDrop],
    ) -> Result<Vec<(SourceSpan, OwnerRelease)>, BackendContractError> {
        let mut releases = Vec::new();
        for drop in drops {
            let action = if drop.moved_fields.is_empty() {
                CleanupAction::DropBinding { binding_index: drop.binding_index }
            } else {
                CleanupAction::DropRemainingFields {
                    binding_index: drop.binding_index,
                    moved_fields: drop.moved_fields.clone(),
                }
            };
            if let Some(owner) = self.action(&action)? {
                releases.push((drop.span, owner));
            }
        }
        Ok(releases)
    }

    /// The release of one action, or `None` when it releases nothing at run
    /// time: a view ends, an arena's storage ends with its frame, and an
    /// owner of a plain data type owns no resource.
    fn action(&self, action: &CleanupAction) -> Result<Option<OwnerRelease>, BackendContractError> {
        let (binding_index, steps, action) = match action {
            CleanupAction::EndBorrow { .. } | CleanupAction::ResetArena { .. } => return Ok(None),
            CleanupAction::DropBinding { binding_index } => {
                let steps = self.owner_steps(*binding_index, &[])?;
                let action = LoweredCleanupInstruction::DropBinding {
                    binding_index: *binding_index,
                    name: self.name(*binding_index)?,
                };
                (*binding_index, steps, action)
            }
            CleanupAction::DropRemainingFields { binding_index, moved_fields } => {
                let steps = self.owner_steps(*binding_index, moved_fields)?;
                let action = LoweredCleanupInstruction::DropBindingFields {
                    binding_index: *binding_index,
                    name: self.name(*binding_index)?,
                    moved_fields: moved_fields.clone(),
                };
                (*binding_index, steps, action)
            }
            CleanupAction::DropPayloadField { binding_index, enum_name, variant, field } => {
                let steps = match self.model.binding_type_names.get(binding_index) {
                    Some(type_name) => {
                        self.layouts.payload_release_steps(type_name, variant, field)?
                    }
                    None => Vec::new(),
                };
                let action = LoweredCleanupInstruction::DropPayloadField {
                    binding_index: *binding_index,
                    name: self.name(*binding_index)?,
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    field: field.clone(),
                };
                (*binding_index, steps, action)
            }
        };
        if steps.is_empty() {
            return Ok(None);
        }
        let name = self.name(binding_index)?;
        Ok(Some(OwnerRelease { binding_index, name, action, steps }))
    }

    fn owner_steps(
        &self,
        binding_index: usize,
        moved_fields: &[String],
    ) -> Result<Vec<CleanupStep>, BackendContractError> {
        let binding = self.model.bindings.get(binding_index).ok_or_else(|| {
            BackendContractError::InvalidCleanupContract(format!(
                "cleanup references invalid binding `{binding_index}`"
            ))
        })?;
        self.layouts.release_steps(
            self.model.binding_type_names.get(&binding_index),
            binding.ty,
            moved_fields,
        )
    }

    fn name(&self, binding_index: usize) -> Result<String, BackendContractError> {
        self.model.bindings.get(binding_index).map(|binding| binding.name.clone()).ok_or_else(
            || {
                BackendContractError::InvalidCleanupContract(format!(
                    "cleanup references invalid binding `{binding_index}`"
                ))
            },
        )
    }
}
