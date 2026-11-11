mod emit;
mod registry;

pub(super) use emit::define_performances;
pub(super) use registry::{
    PerformanceDefinition, PerformanceRegistry, declare_performance_functions, dispatch_key,
};
