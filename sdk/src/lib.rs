#![allow(private_bounds)]
#![allow(async_fn_in_trait)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::wrong_self_convention)]

#[cfg(feature = "l1")]
pub mod l1;
#[cfg(feature = "l2")]
pub mod l2;

pub mod types;

pub fn set_angstrom_constants_with_chain_id(chain_id: u64) -> eyre::Result<()> {
    angstrom_types_primitives::primitive::try_init_with_chain_id(chain_id)
}
