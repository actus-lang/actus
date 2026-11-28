use crate::ast::{Argument, EnumPayload, Expr, Role, StructFieldRole, TypeName};

use crate::semantic::analyzer::{Analyzer, expression_span};
use crate::semantic::errors::{SemanticError, SemanticErrorKind};
use crate::semantic::model::{Origin, OriginRoot};

/// Nesting depth at which a recursive declaration is assumed not to hold a
/// view, so a self-referential struct or enum cannot loop forever.
const VIEW_SEARCH_DEPTH: usize = 8;

impl Analyzer {
    /// Whether a value of this type can hold a view: a struct with an `abs`
    /// field, an enum or generic instance with an `abs` payload or argument
    /// (ADR-0046 2.3), or an aggregate that contains one.
    pub(in crate::semantic) fn type_holds_view(&self, type_name: &TypeName) -> bool {
        self.type_holds_view_within(type_name, 0)
    }

    fn type_holds_view_within(&self, type_name: &TypeName, depth: usize) -> bool {
        if depth > VIEW_SEARCH_DEPTH {
            return false;
        }
        if type_name.arguments.iter().any(|argument| {
            argument.reference_role == Some(Role::Abs)
                || self.type_holds_view_within(argument, depth + 1)
        }) {
            return true;
        }
        if let Some(definition) = self.struct_types.get(&type_name.name) {
            return definition.fields.iter().any(|field| {
                field.role == StructFieldRole::Abs
                    || field.ty.reference_role == Some(Role::Abs)
                    || self.type_holds_view_within(&field.ty, depth + 1)
            });
        }
        let Some(definition) = self.enum_types.get(&type_name.name) else { return false };
        definition.variants.iter().any(|variant| match &variant.payload {
            EnumPayload::Unit => false,
            EnumPayload::Tuple(types) => {
                types.iter().any(|ty| self.type_holds_view_within(ty, depth + 1))
            }
            EnumPayload::Struct(fields) => {
                fields.iter().any(|field| self.type_holds_view_within(&field.ty, depth + 1))
            }
        })
    }

    /// The origin of every view that the value of an expression holds: the
    /// `abs` fields of a struct literal, the `abs` payload of an enum
    /// constructor, the views a binding or a field of it was built with, and
    /// the views that a call's arguments put into its result, and the views of
    /// every arm of an `if` or `case`. A struct or enum that is moved into
    /// another carries its views along.
    pub(in crate::semantic) fn value_view_origins(&self, expression: &Expr) -> Vec<Origin> {
        match expression {
            Expr::Grouping { expression, .. }
            | Expr::Borrow { expression, .. }
            | Expr::Try { expression, .. } => self.value_view_origins(expression),
            Expr::Identifier { .. } => self.held_views_of(expression),
            Expr::FieldAccess { .. } | Expr::Index { .. } => {
                if self.resolved_type_name(expression).is_some_and(|ty| self.type_holds_view(&ty)) {
                    self.held_views_of(expression)
                } else {
                    Vec::new()
                }
            }
            Expr::StructLit { name, fields, .. } => fields
                .iter()
                .flat_map(|initializer| {
                    let is_view = self
                        .struct_field(name, &initializer.name)
                        .is_some_and(|field| field.role == StructFieldRole::Abs);
                    if is_view {
                        self.view_source_origins(&initializer.value)
                    } else {
                        self.value_view_origins(&initializer.value)
                    }
                })
                .collect(),
            Expr::MethodCall { receiver, arguments, .. }
                if self.enum_receiver_name(receiver).is_some() =>
            {
                let stores_views = self.resolved_type_name(expression).is_some_and(|ty| {
                    ty.arguments.iter().any(|a| a.reference_role == Some(Role::Abs))
                });
                arguments
                    .iter()
                    .flat_map(|argument| {
                        if stores_views {
                            self.view_source_origins(&argument.expression)
                        } else {
                            self.value_view_origins(&argument.expression)
                        }
                    })
                    .collect()
            }
            Expr::If { span, .. } | Expr::Case { span, .. } => self.conditional_views(*span),
            Expr::Call { callee, arguments, .. } => {
                self.call_view_origins(expression, callee, None, arguments)
            }
            Expr::MethodCall { receiver, method, arguments, .. } => {
                self.call_view_origins(expression, method, Some(receiver), arguments)
            }
            _ => Vec::new(),
        }
    }

    fn held_views_of(&self, expression: &Expr) -> Vec<Origin> {
        self.root_binding_index(expression)
            .and_then(|index| self.binding_held_views.get(&index))
            .cloned()
            .unwrap_or_default()
    }

    /// The views a call puts into its result when the result can hold one: a
    /// view of every argument given as `abs`, and the views of every
    /// aggregate moved in.
    fn call_view_origins(
        &self,
        call: &Expr,
        callee: &str,
        receiver: Option<&Expr>,
        arguments: &[Argument],
    ) -> Vec<Origin> {
        let Some(signature) = self.signatures.get(callee) else { return Vec::new() };
        if signature.return_access == Some(crate::ast::ReturnAccess::Abs)
            || !self.resolved_type_name(call).is_some_and(|ty| self.type_holds_view(&ty))
        {
            return Vec::new();
        }
        let offset = usize::from(receiver.is_some());
        let receiver_origins = receiver.into_iter().flat_map(|receiver| {
            if signature.params.first().is_some_and(|(_, role, _)| *role == Role::Abs) {
                self.view_source_origins(receiver)
            } else {
                self.value_view_origins(receiver)
            }
        });
        let argument_origins = arguments.iter().enumerate().flat_map(|(position, argument)| {
            let declared = signature.params.iter().enumerate().find(|(slot, (name, _, _))| {
                argument.name.as_ref().map_or(*slot == position + offset, |given| given == name)
            });
            let role = argument.role.clone().or(declared.map(|(_, (_, role, _))| role.clone()));
            if role == Some(Role::Abs) {
                self.view_source_origins(&argument.expression)
            } else {
                self.value_view_origins(&argument.expression)
            }
        });
        receiver_origins.chain(argument_origins).collect()
    }

    /// Freezes the sources of a call whose result can hold a view of them
    /// (ADR-0046 2.3). A callee may return an aggregate that holds a view of
    /// any of its `abs` parameters, and the caller does not track which one,
    /// so every non-scalar argument passed to an `abs` parameter is borrowed
    /// exactly as `Holder { view: source }` would borrow it: frozen until the
    /// end of the block that receives the result, with the same diagnostics
    /// for a later drop, move or mutation. A result that cannot hold a view,
    /// or a `-> abs` result (tracked by its origin), borrows nothing.
    pub(in crate::semantic) fn register_call_result_views(
        &mut self,
        arguments: &[Argument],
        parameter_indices: &[usize],
        signature: &crate::semantic::calls::VerbSignature,
    ) -> Result<(), SemanticError> {
        let holds_view = signature.return_access != Some(crate::ast::ReturnAccess::Abs)
            && signature
                .return_type_name
                .as_ref()
                .is_some_and(|ty| self.type_holds_view(&self.normalize_imported_type_name(ty)));
        if !holds_view {
            return Ok(());
        }
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            let scalar = self
                .expression_type_name(&argument.expression)
                .is_some_and(|name| crate::semantic::calls::shapes::is_scalar_name(&name));
            if signature.params[*parameter_index].1 == Role::Abs && !scalar {
                self.register_view_borrow(
                    &argument.expression,
                    expression_span(&argument.expression),
                )?;
            }
        }
        Ok(())
    }

    /// The origin of the view that an `abs` slot takes of a source. A constant
    /// is static and a view that has no tracked origin comes from a scalar
    /// `abs` parameter; every other source with no `abs` parameter origin is
    /// reported as `Unknown`, which a returned value cannot hold.
    fn view_source_origins(&self, source: &Expr) -> Vec<Origin> {
        let source = match source {
            Expr::Borrow { expression, .. } | Expr::Grouping { expression, .. } => expression,
            other => other,
        };
        match source {
            Expr::StructLit { .. } => self.value_view_origins(source),
            Expr::MethodCall { receiver, .. } if self.enum_receiver_name(receiver).is_some() => {
                self.value_view_origins(source)
            }
            Expr::Identifier { name, .. } if self.is_constant_name(name) => Vec::new(),
            Expr::Integer { .. }
            | Expr::BoolLiteral { .. }
            | Expr::FloatLiteral { .. }
            | Expr::StringLiteral { .. } => Vec::new(),
            _ => match self.origin_of(source) {
                Origin::None
                    if self
                        .root_binding_index(source)
                        .is_some_and(|index| self.model.bindings[index].role == Role::Abs) =>
                {
                    Vec::new()
                }
                Origin::None => vec![Origin::Unknown],
                origin => vec![origin],
            },
        }
    }

    /// A returned value, or one stored through an `ins` parameter, may hold
    /// only views of the verb's own `abs` parameters (ADR-0021, ADR-0046 2.3).
    /// A view of a local or of a value the verb owns would dangle after the
    /// verb returns, so the compiler rejects it at the expression.
    pub(in crate::semantic) fn reject_escaping_views(
        &self,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let escaping =
            self.value_view_origins(expression).into_iter().find_map(|origin| match origin {
                Origin::None | Origin::AbsParameter { .. } | Origin::Derived { .. } => None,
                Origin::Binding { binding_index } => Some(Some(binding_index)),
                Origin::Multiple { roots } => roots.iter().find_map(|root| match root {
                    OriginRoot::Binding(index) => Some(Some(*index)),
                    OriginRoot::Parameter(_) => None,
                }),
                Origin::Unknown => Some(self.root_binding_index(expression)),
            });
        let Some(culprit) = escaping else { return Ok(()) };
        let name = culprit.map_or_else(
            || "temporary value".to_owned(),
            |index| self.model.bindings[index].name.clone(),
        );
        Err(SemanticError {
            kind: SemanticErrorKind::BorrowedReturn { name },
            span: expression_span(expression),
        })
    }

    /// Records the provenance of a binding that is declared or assigned a
    /// value: its arena origin and the views the value holds. Storing views
    /// into an `ins` parameter would let them outlive the verb through the
    /// caller's object, so only views of `abs` parameters may be stored.
    pub(in crate::semantic) fn record_binding_provenance(
        &mut self,
        index: usize,
        value: &Expr,
    ) -> Result<(), SemanticError> {
        self.record_binding_arena_provenance(index, value);
        if self.model.bindings[index].role == Role::Ins {
            self.reject_escaping_views(value)?;
        }
        let origins = self.value_view_origins(value);
        if origins.is_empty() {
            self.binding_held_views.remove(&index);
        } else {
            self.hold_views_in_binding(index, &origins, expression_span(value))?;
            self.binding_held_views.insert(index, origins);
        }
        Ok(())
    }

    /// Records the views that a value assigned to a field adds to its owner.
    pub(in crate::semantic) fn record_field_view_provenance(
        &mut self,
        object: &Expr,
        value: &Expr,
    ) -> Result<(), SemanticError> {
        let origins = self.value_view_origins(value);
        let Some(index) = self.root_binding_index(object).filter(|_| !origins.is_empty()) else {
            return Ok(());
        };
        if self.model.bindings[index].role == Role::Ins {
            self.reject_escaping_views(value)?;
        }
        self.hold_views_in_binding(index, &origins, expression_span(value))?;
        self.binding_held_views.entry(index).or_default().extend(origins);
        Ok(())
    }
}
