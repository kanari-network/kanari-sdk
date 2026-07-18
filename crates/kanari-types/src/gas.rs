#[path = "gas_v1.rs"]
pub mod gas_v1;
#[path = "gas_v2.rs"]
pub mod gas_v2;
#[path = "gas_v3.rs"]
pub mod gas_v3;
#[path = "gas_v3_1.rs"]
pub mod gas_v3_1;

/// Select the active gas implementation here.
///
/// Change the selector to `v1`, `v2`, or `v3` to choose the active pricing
/// policy. All consumers re-export the selected policy from this module.
macro_rules! select_gas_impl {
    (v1) => {
        pub use self::gas_v1::*;
    };
    (v2) => {
        pub use self::gas_v2::*;
    };
    (v3) => {
        pub use self::gas_v3::*;
    };
    (v3_1) => {
        pub use self::gas_v3_1::*;
    };
}

select_gas_impl!(v2);
