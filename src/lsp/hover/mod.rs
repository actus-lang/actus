mod formatting;
mod lookup;
mod model;
mod tokens;

use super::position::LspRange;

#[derive(Clone, Debug)]
pub struct HoverInfo {
    pub contents: String,
    pub range: LspRange,
}

pub(super) use formatting::contract_documentation;
pub(super) use lookup::find_hover;
