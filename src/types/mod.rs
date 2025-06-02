mod common;

pub use common::*;
mod historical_order_filters;
pub use historical_order_filters::*;

pub mod errors;
pub mod fillers;

pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

pub const WETH: alloy_primitives::Address =
    alloy_primitives::address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");

pub const UNI: alloy_primitives::Address =
    alloy_primitives::address!("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984");
