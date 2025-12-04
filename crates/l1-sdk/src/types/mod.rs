mod common;

mod bundle_utils;
pub use bundle_utils::*;
pub use common::*;
mod historical_order_filters;
pub use historical_order_filters::*;

pub mod errors;
pub mod fillers;

pub mod fees;

pub fn set_angstrom_constants_with_chain_id(chain_id: u64) -> eyre::Result<()> {
    angstrom_types_primitives::primitive::try_init_with_chain_id(chain_id)
}
