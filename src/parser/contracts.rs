use crate::ast::{VerbContract, VerbContractSection, VerbContractSectionKind};
use crate::lexer::SourceSpan;

const SECTION_NAMES: [(&str, VerbContractSectionKind); 8] = [
    ("purpose", VerbContractSectionKind::Purpose),
    ("inputs", VerbContractSectionKind::Inputs),
    ("outputs", VerbContractSectionKind::Outputs),
    ("ownership", VerbContractSectionKind::Ownership),
    ("invariants", VerbContractSectionKind::Invariants),
    ("errors", VerbContractSectionKind::Errors),
    ("side_effects", VerbContractSectionKind::SideEffects),
    ("abi", VerbContractSectionKind::Abi),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractParseError {
    ContentBeforePurpose,
    DuplicateSection(String),
    EmptyContract,
    MissingSectionName,
    UnknownSection(String),
}

pub(super) fn parse_contract(
    documentation: Option<&str>,
    span: SourceSpan,
) -> Result<Option<VerbContract>, ContractParseError> {
    let Some(documentation) = documentation else { return Ok(None) };
    let mut lines = documentation.lines();
    let Some(marker) = lines.next().map(str::trim) else { return Ok(None) };
    if marker != "contract:" {
        return Ok(None);
    }

    let mut sections = Vec::new();
    let mut current: Option<(VerbContractSectionKind, String)> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.ends_with(':') && !trimmed.starts_with('-') {
            if let Some((kind, text)) = current.take() {
                sections.push(VerbContractSection { kind, text: normalize_text(&text), span });
            }
            let name = trimmed.trim_end_matches(':');
            if name.is_empty() {
                return Err(ContractParseError::MissingSectionName);
            }
            let Some((_, kind)) = SECTION_NAMES.iter().find(|(candidate, _)| *candidate == name)
            else {
                return Err(ContractParseError::UnknownSection(name.to_owned()));
            };
            if sections.iter().any(|section: &VerbContractSection| section.kind == *kind)
                || current.as_ref().is_some_and(|(current_kind, _)| current_kind == kind)
            {
                return Err(ContractParseError::DuplicateSection(name.to_owned()));
            }
            current = Some((*kind, String::new()));
        } else if let Some((_, text)) = &mut current {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(line.trim_end());
        } else if !trimmed.is_empty() {
            return Err(ContractParseError::ContentBeforePurpose);
        }
    }
    if let Some((kind, text)) = current {
        sections.push(VerbContractSection { kind, text: normalize_text(&text), span });
    }
    if sections.is_empty() {
        return Err(ContractParseError::EmptyContract);
    }
    Ok(Some(VerbContract { sections, span }))
}

fn normalize_text(text: &str) -> String {
    text.trim().to_owned()
}
