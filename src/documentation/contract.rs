use crate::ast::{PackField, Param, ReturnType, Role, StructField, StructFieldRole};

use super::DocumentationSection;

pub(super) struct Contract {
    pub(in crate::documentation) roles: Vec<&'static str>,
    pub(in crate::documentation) has_return: bool,
    pub(in crate::documentation) result_return: bool,
    pub(in crate::documentation) external: bool,
    pub(in crate::documentation) mutation: bool,
    pub(in crate::documentation) allocation: bool,
    pub(in crate::documentation) cleanup: bool,
}

impl Contract {
    pub(super) fn type_definition() -> Self {
        Self {
            roles: Vec::new(),
            has_return: false,
            result_return: false,
            external: false,
            mutation: false,
            allocation: false,
            cleanup: false,
        }
    }

    pub(super) fn field(role: &'static str) -> Self {
        let mut contract = Self::type_definition();
        contract.roles.push(role);
        contract.mutation = role == "ins";
        contract.cleanup = role == "dat";
        contract
    }

    pub(super) fn callable(
        params: &[Param],
        return_type: Option<&ReturnType>,
        external: bool,
    ) -> Self {
        let roles: Vec<&'static str> = params.iter().map(RoleLabel::role_name).collect();
        let result_return = return_type.is_some_and(|return_type| return_type.ty.name == "Result");
        Self {
            mutation: has_resource_loan(params),
            cleanup: has_resource_transfer(params),
            allocation: has_storage_type(params, return_type),
            roles,
            has_return: return_type.is_some(),
            result_return,
            external,
        }
    }

    pub(super) fn required_sections(&self) -> Vec<DocumentationSection> {
        let mut sections = vec![DocumentationSection::Purpose];
        if !self.roles.is_empty() {
            sections.push(DocumentationSection::Ownership);
        }
        if self.has_return {
            sections.push(DocumentationSection::Returns);
        }
        if self.result_return {
            sections.push(DocumentationSection::Errors);
        }
        if self.external {
            sections.push(DocumentationSection::Abi);
        }
        if self.mutation {
            sections.push(DocumentationSection::Mutation);
        }
        if self.allocation {
            sections.push(DocumentationSection::Allocation);
        }
        if self.cleanup {
            sections.push(DocumentationSection::Cleanup);
        }
        sections.sort_by_key(|section| section.label());
        sections.dedup();
        sections
    }
}

fn has_storage_type(params: &[Param], return_type: Option<&ReturnType>) -> bool {
    params.iter().any(|param| is_storage_type(&param.ty.name))
        || return_type.is_some_and(|return_type| is_storage_type(&return_type.ty.name))
}

fn is_storage_type(name: &str) -> bool {
    matches!(name, "Buffer" | "Path")
}

fn has_resource_loan(params: &[Param]) -> bool {
    params.iter().any(|param| param.role == Role::Ins && is_resource_type(&param.ty.name))
}

fn has_resource_transfer(params: &[Param]) -> bool {
    params.iter().any(|param| param.role == Role::Dat && is_resource_type(&param.ty.name))
}

fn is_resource_type(name: &str) -> bool {
    matches!(name, "Buffer" | "Path" | "File" | "Cursor" | "PathComponents" | "OpenOptions")
}

pub(super) fn word_count(text: &str) -> usize {
    text.split_whitespace().filter(|word| word.chars().any(char::is_alphabetic)).count()
}

pub(super) fn is_signature_restatement(doc: &str, declaration: &str) -> bool {
    let normalized_doc = doc.to_ascii_lowercase().replace(['.', ',', ':'], "");
    let normalized_name = declaration.to_ascii_lowercase().replace(['.', ',', ':'], "");
    normalized_doc == normalized_name || normalized_doc == format!("the {normalized_name}")
}

trait RoleLabel {
    fn role_name(&self) -> &'static str;
}

impl RoleLabel for Param {
    fn role_name(&self) -> &'static str {
        match self.role {
            Role::Erg => "erg",
            Role::Abs => "abs",
            Role::Dat => "dat",
            Role::Ins => "ins",
        }
    }
}

pub(super) trait FieldRoleLabel {
    fn role_name(&self) -> &'static str;
}

impl FieldRoleLabel for StructField {
    fn role_name(&self) -> &'static str {
        match self.role {
            StructFieldRole::Value | StructFieldRole::Erg => "erg",
            StructFieldRole::Abs => "abs",
            StructFieldRole::Ins => "ins",
        }
    }
}

impl FieldRoleLabel for PackField {
    fn role_name(&self) -> &'static str {
        match self.role {
            Role::Erg => "erg",
            Role::Abs => "abs",
            Role::Dat => "dat",
            Role::Ins => "ins",
        }
    }
}
