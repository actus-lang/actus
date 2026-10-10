mod generation;
mod migration;

pub(super) use generation::{
    append_builtin_serialization_error, append_generated_serialization_decoders,
    append_generated_serialization_encoders,
};
pub(super) use migration::append_generated_serialization_migrations;
