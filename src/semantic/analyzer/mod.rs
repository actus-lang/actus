mod analysis;
mod bounded;
mod constants;
mod declarations;
mod expressions;
mod for_loops;
mod lifecycle;
mod operator_validation;
mod state;
mod statements;
mod validation;
mod verb;

use crate::semantic::normalize_generated_locals;
use crate::semantic::{calls, cleanup, errors, model};
pub use analysis::analyze;
pub(crate) use analysis::{
    validate_bounded_recursion, validate_stack_limit_dynamic_dispatch,
    validate_stack_limit_native_claims, validate_stack_limit_recursion,
};
pub(crate) use expressions::{analysis::case_branch_type, rules::cast_literal_fits};
pub(crate) use state::{Analyzer, ScopeFrame};
pub(crate) use validation::{
    canonical_type_name, expression_span, is_origin_return_expression, try_type_mismatch,
    type_contains_float, type_name_text_contains_float,
};
