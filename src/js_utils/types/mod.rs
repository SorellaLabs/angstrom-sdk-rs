#![allow(non_snake_case)]

mod data_api;
mod node_api;
mod order_builder;
pub use order_builder::{
    OrderBuilderAddLiquidityArgs, OrderBuilderExactFlashOrderArgs,
    OrderBuilderExactStandingOrderArgs, OrderBuilderPartialFlashOrderArgs,
    OrderBuilderPartialStandingOrderArgs, OrderBuilderRemoveLiquidityArgs,
    OrderBuilderTopOfBlockOrderArgs
};
mod fillers;
pub use fillers::ClientFillerTypes;

pub struct WasmUint<const BITS: usize, const LIMBS: usize> {
    limbs: [u64; LIMBS]
}
