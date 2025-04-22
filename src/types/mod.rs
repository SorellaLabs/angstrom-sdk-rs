mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod errors;
pub mod fillers;

#[cfg(feature = "testnet-sepolia")]
pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_POSITION_MANAGER_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::POSITION_MANAGER_ADDRESS;

#[cfg(feature = "testnet-sepolia")]
pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_CONTROLLER_V1_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::CONTROLLER_V1_ADDRESS;

#[cfg(feature = "testnet-sepolia")]
pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_ANGSTROM_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const ANGSTROM_ADDRESS: alloy_primitives::Address = angstrom_types::primitive::ANGSTROM_ADDRESS;

#[cfg(feature = "testnet-sepolia")]
pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_POOL_MANAGER_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::POOL_MANAGER_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
#[cfg(feature = "testnet-sepolia")]
pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;

#[cfg(not(feature = "testnet-sepolia"))]
pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
#[cfg(feature = "testnet-sepolia")]
pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238");

#[cfg(not(feature = "testnet-sepolia"))]
pub const WETH: alloy_primitives::Address =
    alloy_primitives::address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
#[cfg(feature = "testnet-sepolia")]
pub const WETH: alloy_primitives::Address =
    alloy_primitives::address!("0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14");

#[cfg(not(feature = "testnet-sepolia"))]
pub const UNI: alloy_primitives::Address =
    alloy_primitives::address!("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984");
#[cfg(feature = "testnet-sepolia")]
pub const UNI: alloy_primitives::Address =
    alloy_primitives::address!("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984");

pub const ANGSTROM_DOMAIN: alloy_sol_types::Eip712Domain = alloy_sol_types::eip712_domain!(
    name: "Angstrom",
    version: "v1",
    chain_id: 11155111,
    verifying_contract: ANGSTROM_ADDRESS,
);

// AngstromOrder(TOB(TopOfBlockOrder { quantity_in: 43090292450289573, quantity_out: 2277720546, max_gas_asset0: 1, use_internal: false, asset_in: 0xfff9976782d46cc05630d1f6ebab18b2324d6b14, asset_out: 0x1c7d4b196cb0c7b01d743fbc6116a902379c7238, recipient: 0xa7f1aeb6e43443c683865fdb9e15dd01386c955b, valid_for_block: 8173344, meta: OrderMeta { isEcdsa: true, from: 0xa7f1aeb6e43443c683865fdb9e15dd01386c955b, signature: 0x002dfa922062867a2e4866a0a42e56f5bcdbc39e92feb654f5288661251466c505534648b474a9c21d03bfce3ef7d7a02daea037d92971cadae1dc72d9e23466af } }))
// AngstromOrder(TOB(TopOfBlockOrder { quantity_in: 43090631571067573, quantity_out: 2277720546, max_gas_asset0: 1, use_internal: false, asset_in: 0xfff9976782d46cc05630d1f6ebab18b2324d6b14, asset_out: 0x1c7d4b196cb0c7b01d743fbc6116a902379c7238, recipient: 0xa7f1aeb6e43443c683865fdb9e15dd01386c955b, valid_for_block: 8173378, meta: OrderMeta { isEcdsa: true, from: 0xa7f1aeb6e43443c683865fdb9e15dd01386c955b, signature: 0x002fcfe9f8cc6d4897c55de2570405e4684d4c44a98d1dbd39441b299cc12c26144176b1460cb1e9e9418760a5e3987a9ffe066eea7e5e3f459f14e5220ab8ac03 } }))
// AngstromOrder(TOB(TopOfBlockOrder { quantity_in: 43090709436055573, quantity_out: 2277720546, max_gas_asset0: 1, use_internal: false, asset_in: 0xfff9976782d46cc05630d1f6ebab18b2324d6b14, asset_out: 0x1c7d4b196cb0c7b01d743fbc6116a902379c7238, recipient: 0xa7f1aeb6e43443c683865fdb9e15dd01386c955b, valid_for_block: 8173382, meta: OrderMeta { isEcdsa: true, from: 0xa7f1aeb6e43443c683865fdb9e15dd01386c955b, signature: 0x01a1bb0c74f5e5a4b30a6feb9b2ea88371920f24690c851eb2fc7a12a6f9c391cc6b1fbd343a096b3467185d3d64cd92242663132fe5810d24a8a50e2e434ef817 } }))
