use crate::ast::TypeName;

use super::analyzer::Analyzer;
use super::errors::SemanticError;

impl Analyzer {
    pub(super) fn normalize_array_capacities(
        &self,
        type_name: &mut TypeName,
    ) -> Result<(), SemanticError> {
        self.resolve_type_reference(type_name)?;
        if type_name.name == "Array" && type_name.arguments.len() == 2 {
            self.normalize_array_capacities(&mut type_name.arguments[0])?;
            let capacity = &mut type_name.arguments[1];
            if capacity.name.parse::<u32>().is_err()
                && !self.is_const_generic_parameter(&capacity.name)
                && let Some(value) = super::packs::evaluate_constant_name(
                    &capacity.name,
                    &self.constant_initializers,
                    &mut Vec::new(),
                )
            {
                capacity.name = value.to_string();
            }
            return Ok(());
        }
        for argument in &mut type_name.arguments {
            if argument.name.parse::<u32>().is_ok()
                || self.is_const_generic_parameter(&argument.name)
                || self.constants.contains_key(&argument.name)
            {
                continue;
            }
            self.normalize_array_capacities(argument)?;
        }
        Ok(())
    }
}
