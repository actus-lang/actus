use std::fs;
#[cfg(unix)]
use std::process::Command;

use actus::cli::run_with_args;

#[path = "cli/backend_aggregates.rs"]
mod backend_aggregates;
#[path = "cli/backend_arrays.rs"]
mod backend_arrays;
#[path = "cli/backend_selection.rs"]
mod backend_selection;
#[path = "cli/commands.rs"]
mod commands;
#[path = "cli/control_flow.rs"]
mod control_flow;
