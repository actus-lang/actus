mod fields;
mod wide;

pub(super) use fields::{
    PackedField, cast_integer, emit_packed_field_load, emit_packed_field_store, field_address,
    integer_width,
};
