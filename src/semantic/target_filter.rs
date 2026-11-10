use super::errors::SemanticError;

pub fn filter_program_for_target(
    program: &crate::ast::Program,
    target: &crate::target::TargetSpec,
) -> crate::ast::Program {
    crate::ast::Program {
        file_metadata: program.file_metadata.clone(),
        declarations: program
            .declarations
            .iter()
            .filter(|declaration| declaration_matches_target(declaration, target))
            .cloned()
            .collect(),
    }
}

pub fn filter_program_for_target_checked(
    program: &crate::ast::Program,
    target: &crate::target::TargetSpec,
) -> Result<crate::ast::Program, SemanticError> {
    let filtered = filter_program_for_target(program, target);
    let mut declarations = Vec::new();
    for declaration in &filtered.declarations {
        match declaration {
            crate::ast::TopLevelDecl::Verb(verb) => declarations.push((&verb.metadata, verb.span)),
            crate::ast::TopLevelDecl::ExternalVerb(verb) => {
                declarations.push((&verb.metadata, verb.span));
            }
            crate::ast::TopLevelDecl::Role(role) => {
                declarations
                    .extend(role.methods.iter().map(|method| (&method.metadata, method.span)));
            }
            crate::ast::TopLevelDecl::Perform(perform) => {
                declarations
                    .extend(perform.methods.iter().map(|method| (&method.metadata, method.span)));
            }
            _ => {}
        }
    }
    for (metadata, span) in declarations {
        for attribute in metadata {
            let crate::ast::MetaAttribute::Capability(capability) = attribute else { continue };
            if !target.supports_capability(*capability) {
                return Err(SemanticError {
                    kind: crate::semantic::SemanticErrorKind::UnsupportedCapability {
                        capability: capability.name().to_owned(),
                        target: target.triple().to_string(),
                    },
                    span,
                });
            }
        }
    }
    Ok(filtered)
}

fn declaration_matches_target(
    declaration: &crate::ast::TopLevelDecl,
    target: &crate::target::TargetSpec,
) -> bool {
    let metadata = match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => &verb.metadata,
        crate::ast::TopLevelDecl::ExternalVerb(verb) => &verb.metadata,
        _ => return true,
    };
    metadata.iter().all(|attribute| match attribute {
        crate::ast::MetaAttribute::Target(selector) => target.matches_platform(selector),
        crate::ast::MetaAttribute::Test => true,
        crate::ast::MetaAttribute::Fault(_) => true,
        crate::ast::MetaAttribute::Limitless(_) => true,
        crate::ast::MetaAttribute::MustUse(_) => true,
        crate::ast::MetaAttribute::Effect(_) => true,
        crate::ast::MetaAttribute::Deterministic => true,
        crate::ast::MetaAttribute::Realtime => true,
        crate::ast::MetaAttribute::Bounded(_) => true,
        crate::ast::MetaAttribute::StackLimit(_) => true,
        crate::ast::MetaAttribute::Capability(_) => true,
        crate::ast::MetaAttribute::Repr(_) => true,
        crate::ast::MetaAttribute::Volatile => true,
        crate::ast::MetaAttribute::ZeroOnDrop => true,
    })
}
