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
use crate::semantic::{
    CleanupAction, LoopExitKind, PlaceReplacement, ReplacedPlace, ScopeCleanup, SemanticModel,
    TemporaryRelease,
};

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

/// The release of the value a field or an array element held, run by the
/// assignment that replaces it (ADR-0125). For a field, the steps address the
/// field from the value the lowering holds for the field's owner (`c` in
/// `c.b = ...`); for an element, from the element the lowering loads before
/// it stores the new one. `action` names the binding the place lies in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaceRelease {
    pub(crate) action: LoweredCleanupInstruction,
    pub(crate) steps: Vec<CleanupStep>,
}

/// The owners one scope releases, in reverse declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScopeRelease {
    pub(crate) span: SourceSpan,
    pub(crate) owners: Vec<OwnerRelease>,
    /// The arenas the scope releases after its owners (ADR-0030, ADR-0125).
    pub(crate) arenas: Vec<ArenaRelease>,
}

/// The release of an arena owner: its placements end in one bulk step that
/// empties the arena, by setting the offset word of `width` bytes that
/// follows its `capacity` bytes of storage back to zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArenaRelease {
    pub(crate) name: String,
    pub(crate) capacity: u32,
    pub(crate) width: u32,
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
    places: Vec<(SourceSpan, PlaceRelease)>,
    temporaries: Vec<(SourceSpan, SourceSpan, PlaceRelease)>,
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

    /// What the assignment to a field or an element at `span` releases
    /// before it stores the new value.
    pub(crate) fn place_replacement(&self, span: SourceSpan) -> Option<&PlaceRelease> {
        self.places.iter().find(|(at, _)| *at == span).map(|(_, place)| place)
    }

    /// The release of the temporary produced by the expression at `span`.
    pub(crate) fn temporary(&self, span: SourceSpan) -> Option<&PlaceRelease> {
        self.temporaries.iter().find(|(at, _, _)| *at == span).map(|(_, _, release)| release)
    }

    /// The temporaries released once the expression at `span` is evaluated,
    /// each with the span of the expression that produced it.
    pub(crate) fn temporaries_released_after(
        &self,
        span: SourceSpan,
    ) -> Vec<(SourceSpan, &PlaceRelease)> {
        self.temporaries
            .iter()
            .filter(|(_, after, _)| *after == span)
            .map(|(at, _, release)| (*at, release))
            .collect()
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
                places: resolver.places(&model.place_replacements[record.places.clone()])?,
                temporaries: resolver
                    .temporaries(&model.temporary_releases[record.temporaries.clone()])?,
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
        let mut arenas = Vec::new();
        for action in &scope.actions {
            if let CleanupAction::ResetArena { binding_index } = action {
                arenas.extend(self.arena(*binding_index)?);
            } else if let Some(owner) = self.action(action)? {
                owners.push(owner);
            }
        }
        Ok(ScopeRelease { span: scope.span, owners, arenas })
    }

    /// The release of the arena owner `binding_index`, from the capacity its
    /// type names and the pointer width of the target; `None` when the type
    /// names no concrete capacity.
    fn arena(&self, binding_index: usize) -> Result<Option<ArenaRelease>, BackendContractError> {
        let Some(type_name) = self.model.binding_type_names.get(&binding_index) else {
            return Ok(None);
        };
        let Some(capacity) =
            type_name.arguments.first().and_then(|capacity| capacity.name.parse::<u32>().ok())
        else {
            return Ok(None);
        };
        let Some(layout) =
            crate::codegen::BackendLayoutProvider::layout_for(self.layouts, type_name)
        else {
            return Ok(None);
        };
        let name = self.name(binding_index)?;
        Ok(Some(ArenaRelease { name, capacity, width: layout.size }))
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

    fn places(
        &self,
        places: &[PlaceReplacement],
    ) -> Result<Vec<(SourceSpan, PlaceRelease)>, BackendContractError> {
        let mut releases = Vec::new();
        for place in places {
            let steps = match &place.place {
                ReplacedPlace::Field { owner_type, field, moved_fields } => {
                    self.layouts.field_release_steps(owner_type, field, moved_fields)?
                }
                ReplacedPlace::Element { element_type } => {
                    self.layouts.release_steps(Some(element_type), None, &[])?
                }
            };
            if steps.is_empty() {
                continue;
            }
            let action = LoweredCleanupInstruction::DropBinding {
                binding_index: place.binding_index,
                name: self.name(place.binding_index)?,
            };
            releases.push((place.span, PlaceRelease { action, steps }));
        }
        Ok(releases)
    }

    /// The release of each temporary, addressed from the value of the
    /// expression that produced it. A temporary belongs to no binding, so its
    /// action names none (`usize::MAX`); the action only tells the back ends
    /// that a value is released.
    fn temporaries(
        &self,
        temporaries: &[TemporaryRelease],
    ) -> Result<Vec<(SourceSpan, SourceSpan, PlaceRelease)>, BackendContractError> {
        let mut releases = Vec::new();
        for temporary in temporaries {
            let steps = self.layouts.release_steps(Some(&temporary.type_name), None, &[])?;
            if steps.is_empty() {
                continue;
            }
            let action = LoweredCleanupInstruction::DropBinding {
                binding_index: usize::MAX,
                name: "temporary".to_owned(),
            };
            let release = PlaceRelease { action, steps };
            releases.push((temporary.span, temporary.release_after, release));
        }
        Ok(releases)
    }

    /// The release of one action, or `None` when it releases nothing at run
    /// time: a view ends, and an owner of a plain data type owns no resource.
    /// An arena is released by `arena`.
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
