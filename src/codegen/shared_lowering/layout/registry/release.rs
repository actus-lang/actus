//! The cleanup of one value as a list of bridge calls (ADR-0125).
//!
//! A value is released by, in this order, its own drop contract, the release
//! of each field it still owns in reverse declaration order (recursively),
//! and the storage that holds it. An enum releases the payload of its active
//! variant only: every payload step carries a guard on the variant tag the
//! value holds. An array releases its elements, last element first. Each step
//! addresses its part of the value by an offset from the owner's root, so a
//! step never needs a value of its own.

use super::*;
use crate::codegen::shared_lowering::exits::{CleanupGuard, CleanupStep};

/// How deep a value's parts are followed; a self-referential declaration
/// cannot be stored by value, so this only bounds a malformed program.
const RELEASE_DEPTH: usize = 8;

/// Where the parts of one owner are collected: the owner's layout, which every
/// field step is relative to, and the steps found so far.
struct ReleaseWalk {
    root_layout: BackendLayout,
    steps: Vec<CleanupStep>,
}

impl ProgramLayouts<'_> {
    /// The steps that release the value of an owner of `type_name` (or of the
    /// builtin `builtin`), except the field paths in `moved_fields`, which
    /// were moved out before the release (ADR-0046). A partially moved owner
    /// does not run its own drop contract, because it no longer holds a whole
    /// value, but its storage is still released.
    pub(in crate::codegen::shared_lowering) fn release_steps(
        &self,
        type_name: Option<&TypeName>,
        builtin: Option<BuiltinType>,
        moved_fields: &[String],
    ) -> Result<Vec<CleanupStep>, BackendContractError> {
        let root = type_name.map(|type_name| self.normalize_type_name(type_name));
        if builtin == Some(BuiltinType::Buffer)
            || root.as_ref().is_some_and(|root| root.name == "Buffer")
        {
            return Ok(vec![unguarded(self.resource_cleanup(BackendCleanupBridge::BufferDrop))]);
        }
        let Some(root) =
            root.filter(|root| !matches!(root.reference_role, Some(Role::Abs | Role::Ins)))
        else {
            return Ok(Vec::new());
        };
        if root.name == "Region" {
            return Ok(vec![unguarded(self.resource_cleanup(BackendCleanupBridge::RegionDrop))]);
        }
        let Some(root_layout) = self.value_layout(&root) else { return Ok(Vec::new()) };
        let mut walk = ReleaseWalk { root_layout, steps: Vec::new() };
        self.collect_release(&mut walk, &root, 0, &[], (moved_fields, ""), 0);
        walk.steps.extend(self.release_storage(&root, root_layout));
        validated(walk.steps)
    }

    /// The steps that release the payload field `field` of the variant
    /// `variant` of an enum of `enum_type`, which a `case dat` branch took
    /// over without binding the field. The variant `_` releases the whole
    /// value by its active variant (a wildcard branch).
    pub(in crate::codegen::shared_lowering) fn payload_release_steps(
        &self,
        enum_type: &TypeName,
        variant: &str,
        field: &str,
    ) -> Result<Vec<CleanupStep>, BackendContractError> {
        if variant == "_" {
            return self.release_steps(Some(enum_type), None, &[]);
        }
        let enum_type = self.normalize_type_name(enum_type);
        let Some(layout) = self.enum_layout_for(&enum_type) else { return Ok(Vec::new()) };
        let Some(fields) = self.enum_variant_payload_fields(&enum_type, variant) else {
            return Ok(Vec::new());
        };
        let mut walk = ReleaseWalk { root_layout: layout.layout, steps: Vec::new() };
        for payload in fields.iter().filter(|payload| payload.name == field) {
            let offset = layout.payload_offset + payload.offset;
            self.collect_release(&mut walk, &payload.ty, offset, &[], (&[], ""), 0);
        }
        validated(walk.steps)
    }

    /// Adds the steps that release the value of `type_name` stored at
    /// `offset` in the owner. `guards` are the variant tests every step of
    /// this part needs; `moved` holds the moved paths and the path of this
    /// part inside the owner.
    fn collect_release(
        &self,
        walk: &mut ReleaseWalk,
        type_name: &TypeName,
        offset: u32,
        guards: &[CleanupGuard],
        moved: (&[String], &str),
        depth: usize,
    ) {
        let type_name = self.normalize_type_name(type_name);
        if depth > RELEASE_DEPTH || matches!(type_name.reference_role, Some(Role::Abs | Role::Ins))
        {
            return;
        }
        match type_name.name.as_str() {
            "Buffer" | "Region" => {
                let bridge = if type_name.name == "Buffer" {
                    BackendCleanupBridge::BufferDrop
                } else {
                    BackendCleanupBridge::RegionDrop
                };
                let field = self.field_slot(offset, self.pointer_layout());
                let descriptor = self.cleanup_descriptor(bridge, walk.root_layout, Some(field));
                walk.steps.push(CleanupStep { descriptor, guards: guards.to_vec() });
            }
            "Array" => self.collect_array_release(walk, &type_name, offset, guards, depth),
            _ if self.enum_layout_for(&type_name).is_some()
                && (self.enums.contains_key(type_name.name.as_str())
                    || matches!(type_name.name.as_str(), "Option" | "Result")) =>
            {
                self.collect_enum_release(walk, &type_name, offset, guards, depth);
            }
            _ if self.structs.contains_key(type_name.name.as_str()) => {
                self.collect_struct_release(walk, &type_name, offset, guards, moved, depth);
            }
            _ => self.push_drop_contract(walk, &type_name, offset, guards),
        }
    }

    /// Releases the elements of a fixed array, the last element first.
    fn collect_array_release(
        &self,
        walk: &mut ReleaseWalk,
        type_name: &TypeName,
        offset: u32,
        guards: &[CleanupGuard],
        depth: usize,
    ) {
        let [element, capacity] = type_name.arguments.as_slice() else { return };
        let Ok(capacity) = capacity.name.parse::<u32>() else { return };
        let Some(layout) = self.value_layout(element) else { return };
        for index in (0..capacity).rev() {
            let element_offset = offset + index * layout.size;
            self.collect_release(walk, element, element_offset, guards, (&[], ""), depth + 1);
        }
    }

    /// Releases an enum: its own drop contract, then the payload of every
    /// variant, each guarded by that variant's tag, so only the active
    /// variant's payload is released.
    fn collect_enum_release(
        &self,
        walk: &mut ReleaseWalk,
        type_name: &TypeName,
        offset: u32,
        guards: &[CleanupGuard],
        depth: usize,
    ) {
        let Some(layout) = self.enum_layout_for(type_name) else { return };
        if layout.niche_pointer {
            return;
        }
        self.push_drop_contract(walk, type_name, offset, guards);
        for (variant, discriminant) in self.enum_variants(type_name) {
            let Some(fields) = self.enum_variant_payload_fields(type_name, &variant) else {
                continue;
            };
            let mut variant_guards = guards.to_vec();
            variant_guards.push(CleanupGuard {
                offset: offset + layout.discriminant_offset,
                size: layout.discriminant_size,
                discriminant,
            });
            for field in fields.iter().rev() {
                let field_offset = offset + layout.payload_offset + field.offset;
                let next = depth + 1;
                self.collect_release(
                    walk,
                    &field.ty,
                    field_offset,
                    &variant_guards,
                    (&[], ""),
                    next,
                );
            }
        }
    }

    /// Every variant of an enum with the tag that selects it.
    fn enum_variants(&self, type_name: &TypeName) -> Vec<(String, u32)> {
        match type_name.name.as_str() {
            "Result" => vec![("Ok".to_owned(), 0), ("Err".to_owned(), 1)],
            "Option" => ["Some", "None"]
                .into_iter()
                .filter_map(|variant| {
                    self.enum_variant(type_name, variant).map(|(tag, _)| (variant.to_owned(), tag))
                })
                .collect(),
            _ => self
                .enums
                .get(type_name.name.as_str())
                .map(|definition| {
                    definition
                        .variants
                        .iter()
                        .enumerate()
                        .map(|(index, variant)| (variant.name.clone(), index as u32))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Releases a struct: its own drop contract unless a part of it was moved
    /// out, then every field it still owns in reverse declaration order. A
    /// view field (`abs` or `ins`) owns nothing.
    fn collect_struct_release(
        &self,
        walk: &mut ReleaseWalk,
        type_name: &TypeName,
        offset: u32,
        guards: &[CleanupGuard],
        (moved, path): (&[String], &str),
        depth: usize,
    ) {
        let Some(definition) = self.structs.get(type_name.name.as_str()) else { return };
        let inside = |candidate: &String| {
            path.is_empty() || candidate == path || candidate.starts_with(&format!("{path}."))
        };
        if !moved.iter().any(inside) {
            self.push_drop_contract(walk, type_name, offset, guards);
        }
        for field in definition.fields.iter().rev() {
            if matches!(field.role, StructFieldRole::Abs | StructFieldRole::Ins) {
                continue;
            }
            let field_path =
                if path.is_empty() { field.name.clone() } else { format!("{path}.{}", field.name) };
            if moved.contains(&field_path) {
                continue;
            }
            let Some(slot) = self.field_for(type_name, &field.name) else { continue };
            let field_offset = offset + slot.offset;
            let next = depth + 1;
            self.collect_release(walk, &slot.ty, field_offset, guards, (moved, &field_path), next);
        }
    }

    /// Adds a call of the `Drop` performance of `type_name`, when it has one,
    /// on the part of the owner at `offset`.
    fn push_drop_contract(
        &self,
        walk: &mut ReleaseWalk,
        type_name: &TypeName,
        offset: u32,
        guards: &[CleanupGuard],
    ) {
        let Some(symbol) = self.drop_symbols.get(&self.lookup_key(type_name)) else { return };
        let Some(layout) = self.value_layout(type_name) else { return };
        let (argument, field) = if offset == 0 && layout == walk.root_layout {
            (BackendCleanupArgument::Address, None)
        } else {
            (BackendCleanupArgument::FieldAddress, Some(self.field_slot(offset, layout)))
        };
        walk.steps.push(CleanupStep {
            descriptor: BackendCleanupResolvedDescriptor {
                descriptor: BackendCleanupBridgeDescriptor {
                    bridge: BackendCleanupBridge::PerformanceDrop,
                    argument,
                    result: BackendCleanupResult::Ignored,
                },
                layout: walk.root_layout,
                field,
                performance_symbol: Some(symbol.clone()),
            },
            guards: guards.to_vec(),
        });
    }

    /// The release of the storage of a whole owner, after its parts: a secret
    /// is zeroed and a large aggregate's heap block is freed.
    fn release_storage(&self, root: &TypeName, layout: BackendLayout) -> Option<CleanupStep> {
        if self
            .structs
            .get(root.name.as_str())
            .is_some_and(|definition| definition.is_zero_on_drop())
        {
            let bridge = BackendCleanupBridge::SecureZero { size: layout.size };
            return Some(unguarded(self.cleanup_descriptor(bridge, layout, None)));
        }
        if layout.size < crate::codegen::LARGE_AGGREGATE_THRESHOLD {
            return None;
        }
        let bridge =
            BackendCleanupBridge::AggregateDrop { size: layout.size, alignment: layout.alignment };
        Some(unguarded(self.cleanup_descriptor(bridge, layout, None)))
    }

    /// The layout of a value of `type_name`: a `Result`, which has no layout
    /// of its own in the type table, takes the layout of its enum.
    fn value_layout(&self, type_name: &TypeName) -> Option<BackendLayout> {
        self.type_layout(type_name, &mut Vec::new())
            .or_else(|| self.enum_layout_for(type_name).map(|layout| layout.layout))
    }

    fn field_slot(&self, offset: u32, layout: BackendLayout) -> BackendCleanupField {
        BackendCleanupField { offset, size: layout.size, alignment: layout.alignment }
    }
}

fn unguarded(descriptor: BackendCleanupResolvedDescriptor) -> CleanupStep {
    CleanupStep { descriptor, guards: Vec::new() }
}

fn validated(steps: Vec<CleanupStep>) -> Result<Vec<CleanupStep>, BackendContractError> {
    for step in &steps {
        step.descriptor.validate()?;
    }
    Ok(steps)
}
