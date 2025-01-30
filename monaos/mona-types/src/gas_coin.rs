use move_core_types::{
    annotated_value::MoveStructLayout,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag},
};



/// The number of Mist per KARI token
pub const MIST_PER_KARI: u64 = 1_000_000_000;

/// Total supply denominated in KARI
pub const TOTAL_SUPPLY_KARI: u64 = 100_000_000;

// Note: cannot use checked arithmetic here since `const unwrap` is still unstable.
/// Total supply denominated in Mist
pub const TOTAL_SUPPLY_MIST: u64 = TOTAL_SUPPLY_KARI * MIST_PER_KARI;

pub const GAS_MODULE_NAME: &IdentStr = ident_str!("kari");
pub const GAS_STRUCT_NAME: &IdentStr = ident_str!("kari");