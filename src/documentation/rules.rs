use crate::ast::{
    EnumDef, EnumField, ExternalVerbDecl, PackDecl, Param, PerformDecl, Program, ReturnType,
    RoleDecl, StructDef, TopLevelDecl, VerbDecl,
};
use crate::diagnostics::{
    STRICT_DOCUMENTATION_CONTRADICTION, STRICT_DOCUMENTATION_MISREPRESENTATION,
    STRICT_DOCUMENTATION_MISSING, STRICT_DOCUMENTATION_RESTATEMENT, STRICT_DOCUMENTATION_SECTION,
};
use crate::lexer::SourceSpan;

use super::contract::{Contract, FieldRoleLabel, is_signature_restatement, word_count};
use super::language::{contains_section, has_contradiction, is_misrepresenting};
use super::{DocumentationIssue, DocumentationSection};

/// Validate every public declaration and public member in one parsed program.
pub fn validate_public_documentation(program: &Program) -> Vec<DocumentationIssue> {
    let mut issues = Vec::new();
    for declaration in &program.declarations {
        validate_top_level(declaration, &mut issues);
    }
    issues
}

fn validate_top_level(declaration: &TopLevelDecl, issues: &mut Vec<DocumentationIssue>) {
    match declaration {
        TopLevelDecl::Verb(verb) if verb.is_open => validate_verb(verb, issues),
        TopLevelDecl::ExternalVerb(verb) if verb.is_open => validate_external(verb, issues),
        TopLevelDecl::Struct(definition) if definition.is_open => {
            validate_struct(definition, issues)
        }
        TopLevelDecl::Pack(definition) if definition.is_open => validate_pack(definition, issues),
        TopLevelDecl::Enum(definition) if definition.is_open => validate_enum(definition, issues),
        TopLevelDecl::Role(role) if role.is_open => validate_role(role, issues),
        TopLevelDecl::Perform(perform) if perform.is_open => validate_perform(perform, issues),
        TopLevelDecl::OpenSibling(sibling) => check_doc(
            sibling.doc.as_deref(),
            format!("facade export {}", sibling.name),
            sibling.span,
            Contract::type_definition(),
            issues,
        ),
        _ => {}
    }
}

fn validate_verb(verb: &VerbDecl, issues: &mut Vec<DocumentationIssue>) {
    check_callable(
        &verb.name,
        verb.doc.as_deref(),
        verb.span,
        &verb.params,
        verb.return_type.as_ref(),
        false,
        issues,
    );
}

fn validate_external(verb: &ExternalVerbDecl, issues: &mut Vec<DocumentationIssue>) {
    check_callable(
        &verb.name,
        verb.doc.as_deref(),
        verb.span,
        &verb.params,
        verb.return_type.as_ref(),
        true,
        issues,
    );
}

fn validate_struct(definition: &StructDef, issues: &mut Vec<DocumentationIssue>) {
    check_doc(
        definition.doc.as_deref(),
        format!("struct {}", definition.name),
        definition.span,
        Contract::type_definition(),
        issues,
    );
    for field in &definition.fields {
        let contract = match field.role {
            crate::ast::StructFieldRole::Value => Contract::type_definition(),
            _ => Contract::field(field.role_name()),
        };
        check_doc(
            field.doc.as_deref(),
            format!("field {}.{}", definition.name, field.name),
            field.span,
            contract,
            issues,
        );
    }
}

fn validate_pack(definition: &PackDecl, issues: &mut Vec<DocumentationIssue>) {
    check_doc(
        definition.doc.as_deref(),
        format!("pack {}", definition.name),
        definition.span,
        Contract::type_definition(),
        issues,
    );
    check_doc(
        definition.storage_doc.as_deref(),
        format!("storage field {}.{}", definition.name, definition.storage_name),
        definition.storage.span,
        Contract::field("erg"),
        issues,
    );
    for field in &definition.fields {
        check_doc(
            field.doc.as_deref(),
            format!("pack field {}.{}", definition.name, field.name),
            field.span,
            Contract::field(field.role_name()),
            issues,
        );
    }
}

fn validate_enum(definition: &EnumDef, issues: &mut Vec<DocumentationIssue>) {
    check_doc(
        definition.doc.as_deref(),
        format!("enum {}", definition.name),
        definition.span,
        Contract::type_definition(),
        issues,
    );
    for variant in &definition.variants {
        check_doc(
            variant.doc.as_deref(),
            format!("variant {}.{}", definition.name, variant.name),
            variant.span,
            Contract::type_definition(),
            issues,
        );
        if let crate::ast::EnumPayload::Struct(fields) = &variant.payload {
            for field in fields {
                validate_enum_field(field, &definition.name, &variant.name, issues);
            }
        }
    }
}

fn validate_enum_field(
    field: &EnumField,
    owner: &str,
    variant: &str,
    issues: &mut Vec<DocumentationIssue>,
) {
    check_doc(
        field.doc.as_deref(),
        format!("variant field {}.{}.{}", owner, variant, field.name),
        field.span,
        Contract::type_definition(),
        issues,
    );
}

fn validate_role(role: &RoleDecl, issues: &mut Vec<DocumentationIssue>) {
    check_doc(
        role.doc.as_deref(),
        format!("role {}", role.name),
        role.span,
        Contract::type_definition(),
        issues,
    );
    for method in &role.methods {
        check_callable(
            &format!("{}.{}", role.name, method.name),
            method.doc.as_deref(),
            method.span,
            &method.params,
            method.return_type.as_ref(),
            false,
            issues,
        );
    }
}

fn validate_perform(perform: &PerformDecl, issues: &mut Vec<DocumentationIssue>) {
    check_doc(
        perform.doc.as_deref(),
        format!("performance {} for {}", perform.role_name, perform.target.name),
        perform.span,
        Contract::type_definition(),
        issues,
    );
    for method in &perform.methods {
        validate_verb(method, issues);
    }
}

fn check_callable(
    name: &str,
    doc: Option<&str>,
    span: SourceSpan,
    params: &[Param],
    return_type: Option<&ReturnType>,
    external: bool,
    issues: &mut Vec<DocumentationIssue>,
) {
    let contract = Contract::callable(params, return_type, external);
    check_doc(doc, format!("verb {name}"), span, contract, issues);
}

fn check_doc(
    doc: Option<&str>,
    declaration: String,
    span: SourceSpan,
    contract: Contract,
    issues: &mut Vec<DocumentationIssue>,
) {
    let Some(doc) = doc.map(str::trim).filter(|doc| !doc.is_empty()) else {
        issues.push(issue(
            STRICT_DOCUMENTATION_MISSING,
            declaration,
            DocumentationSection::Purpose,
            span,
            "missing or empty block docstring",
        ));
        return;
    };
    if word_count(doc) < 4 || is_signature_restatement(doc, &declaration) {
        issues.push(issue(
            STRICT_DOCUMENTATION_RESTATEMENT,
            declaration.clone(),
            DocumentationSection::Purpose,
            span,
            "documentation is an incomplete or signature-restating summary",
        ));
    }
    for section in contract.required_sections() {
        if !contains_section(doc, section, &contract) {
            issues.push(issue(
                STRICT_DOCUMENTATION_SECTION,
                declaration.clone(),
                section,
                span,
                format!("documentation does not describe the {} contract", section.label()),
            ));
        }
    }
    if has_contradiction(doc, &contract) {
        issues.push(issue(
            STRICT_DOCUMENTATION_CONTRADICTION,
            declaration.clone(),
            DocumentationSection::Ownership,
            span,
            "documentation contradicts the declaration ownership contract",
        ));
    }
    if is_misrepresenting(doc, &contract) {
        issues.push(issue(
            STRICT_DOCUMENTATION_MISREPRESENTATION,
            declaration,
            DocumentationSection::Errors,
            span,
            "documentation promises behavior that the declared result contract cannot guarantee",
        ));
    }
}

fn issue(
    code: &'static str,
    declaration: String,
    section: DocumentationSection,
    span: SourceSpan,
    message: impl Into<String>,
) -> DocumentationIssue {
    DocumentationIssue { code, declaration, section, span, message: message.into() }
}
