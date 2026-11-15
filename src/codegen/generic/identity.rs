//! Generic verb matching by canonical declaration identity (ADR-0108).
//!
//! An instance names the generic verb it specializes by identity, so two
//! same-named verbs of different modules, or one verb reached through several
//! import bindings, are never confused by spelling. Declarations without an
//! identity (compiler-generated verbs) match by their exact name only.

use crate::ast::DeclarationId;
use crate::semantic::GenericInstance;

/// Whether `instance` specializes the verb declared as `name` with `identity`.
pub(super) fn names(
    instance: &GenericInstance,
    identity: Option<DeclarationId>,
    name: &str,
) -> bool {
    match (instance.verb, identity) {
        (Some(instance_verb), Some(identity)) => instance_verb == identity,
        _ => instance.name == name,
    }
}

/// Whether `instance` is called from the body of the generic verb that
/// `caller` specializes.
pub(super) fn called_from(instance: &GenericInstance, caller: &GenericInstance) -> bool {
    match (instance.caller_verb, caller.verb) {
        (Some(caller_verb), Some(verb)) => caller_verb == verb,
        _ => instance.caller.as_deref() == Some(caller.name.as_str()),
    }
}

/// The generic parameters of the verb `instance` specializes.
pub(super) fn generic_parameters_for<'a>(
    program: &'a crate::ast::Program,
    instance: &GenericInstance,
) -> Option<&'a [crate::ast::GenericParam]> {
    program.declarations.iter().find_map(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) if names(instance, verb.identity, &verb.name) => {
            Some(verb.generic_parameters.as_slice())
        }
        crate::ast::TopLevelDecl::ExternalVerb(verb)
            if names(instance, verb.identity, &verb.name) =>
        {
            Some(verb.generic_parameters.as_slice())
        }
        _ => None,
    })
}

pub(super) fn matches_verb(instance: &GenericInstance, verb: &crate::ast::VerbDecl) -> bool {
    names(instance, verb.identity, &verb.name)
        && instance.arguments.len() == verb.generic_parameters.len()
        && instance.arguments.iter().all(|argument| {
            !super::verbs::contains_generic_parameter(argument, &verb.generic_parameters)
        })
}

pub(super) fn matches_external(
    instance: &GenericInstance,
    verb: &crate::ast::ExternalVerbDecl,
) -> bool {
    names(instance, verb.identity, &verb.name)
        && instance.arguments.len() == verb.generic_parameters.len()
        && instance.arguments.iter().all(|argument| {
            !super::verbs::contains_generic_parameter(argument, &verb.generic_parameters)
        })
}

/// The generic instances of the verb `name` declared by `program`: an
/// imported unit's verb and its instances are paired by identity, never by
/// the last segment of an instance's spelling.
pub(crate) fn instances_of_verb<'a>(
    program: &crate::ast::Program,
    name: &str,
    instances: &'a [GenericInstance],
) -> Vec<&'a GenericInstance> {
    let Some(identity) = program.declarations.iter().find_map(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) if verb.name == name => verb.identity,
        _ => None,
    }) else {
        return Vec::new();
    };
    instances.iter().filter(|instance| instance.verb == Some(identity)).collect()
}

#[cfg(test)]
mod tests {
    use super::{called_from, names};
    use crate::ast::{DeclarationId, DeclarationIdentity, TypeIdentity};
    use crate::semantic::GenericInstance;

    fn id(name: &str) -> DeclarationId {
        DeclarationId::intern(DeclarationIdentity::Builtin { name: name.to_owned() })
    }

    fn instance(
        name: &str,
        verb: Option<DeclarationId>,
        caller: Option<(&str, DeclarationId)>,
    ) -> GenericInstance {
        GenericInstance {
            name: name.to_owned(),
            arguments: Vec::new(),
            identity: TypeIdentity::from_verb_application(name, &[]),
            canonical_key: name.to_owned(),
            caller: caller.map(|(name, _)| name.to_owned()),
            call_span: crate::lexer::SourceSpan::new(0, 0),
            verb,
            caller_verb: caller.map(|(_, verb)| verb),
        }
    }

    /// Two verbs spelled `wrap` in different modules are different verbs; one
    /// verb spelled `a::wrap` through an alias is the same verb.
    #[test]
    fn instances_match_verbs_by_identity_not_spelling() {
        let alpha = id("alpha::wrap");
        let beta = id("beta::wrap");
        assert!(names(&instance("a::wrap", Some(alpha), None), Some(alpha), "wrap"));
        assert!(!names(&instance("wrap", Some(alpha), None), Some(beta), "wrap"));
        assert!(names(&instance("wrap__Int", None, None), None, "wrap__Int"));
        assert!(!names(&instance("a::wrap", None, None), None, "wrap"));

        let caller = instance("a::outer", Some(id("alpha::outer")), None);
        let nested = instance("wrap", Some(alpha), Some(("outer", id("alpha::outer"))));
        let other = instance("wrap", Some(alpha), Some(("outer", id("beta::outer"))));
        assert!(called_from(&nested, &caller));
        assert!(!called_from(&other, &caller));
    }
}
