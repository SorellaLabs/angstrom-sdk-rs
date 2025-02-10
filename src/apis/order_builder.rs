use alloy_dyn_abi::Eip712Domain;
use alloy_primitives::{
    aliases::{I24, U40},
    Address, TxKind, U256
};
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::SolCall;
use angstrom_types::{
    contract_bindings::pool_gate::PoolGate,
    matching::Ray,
    primitive::ANGSTROM_DOMAIN,
    sol_bindings::rpc_orders::{
        ExactFlashOrder, ExactStandingOrder, PartialFlashOrder, PartialStandingOrder,
        TopOfBlockOrder
    }
};

use crate::types::{TransactionRequestWithLiquidityMeta, POOL_GATE_ADDRESS};

pub fn add_liquidity(
    token0: Address,
    token1: Address,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: U256,
    max_fee_per_gas: Option<u128>,
    max_priority_fee_per_gas: Option<u128>
) -> TransactionRequestWithLiquidityMeta {
    let call = PoolGate::addLiquidityCall {
        asset0: token0,
        asset1: token1,
        tickLower: I24::unchecked_from(tick_lower),
        tickUpper: I24::unchecked_from(tick_upper),
        liquidity,
        salt: Default::default()
    };

    let tx = TransactionRequest {
        to: Some(TxKind::Call(POOL_GATE_ADDRESS)),
        input: TransactionInput::both(call.abi_encode().into()),
        max_fee_per_gas,
        max_priority_fee_per_gas,
        ..Default::default()
    };

    TransactionRequestWithLiquidityMeta::new_add_liqudity(tx, call)
}

pub fn remove_liquidity(
    token0: Address,
    token1: Address,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: U256,
    max_fee_per_gas: Option<u128>,
    max_priority_fee_per_gas: Option<u128>
) -> TransactionRequestWithLiquidityMeta {
    let call = PoolGate::removeLiquidityCall {
        asset0: token0,
        asset1: token1,
        tickLower: I24::unchecked_from(tick_lower),
        tickUpper: I24::unchecked_from(tick_upper),
        liquidity,
        salt: Default::default()
    };

    let tx = TransactionRequest {
        to: Some(TxKind::Call(POOL_GATE_ADDRESS)),
        input: TransactionInput::both(call.abi_encode().into()),
        max_fee_per_gas,
        max_priority_fee_per_gas,
        ..Default::default()
    };

    TransactionRequestWithLiquidityMeta::new_remove_liqudity(tx, call)
}

pub fn top_of_block_order(
    asset_in: Address,
    asset_out: Address,
    quantity_in: u128,
    quantity_out: u128,
    max_gas_asset0: u128,
    valid_for_block: u64,
    recipient: Address
) -> TopOfBlockOrder {
    TopOfBlockOrder {
        asset_in,
        asset_out,
        quantity_in,
        quantity_out,
        valid_for_block,
        max_gas_asset0,
        recipient,
        ..Default::default()
    }
}

pub fn partial_standing_order(
    asset_in: Address,
    asset_out: Address,
    min_amount_in: u128,
    max_amount_in: u128,
    min_price: U256,
    deadline: Option<u64>,
    recipient: Address
) -> PartialStandingOrder {
    PartialStandingOrder {
        asset_in,
        asset_out,
        max_amount_in,
        min_amount_in,
        min_price,
        max_extra_fee_asset0: max_amount_in,
        deadline: deadline.map(|d| U40::from(d)).unwrap_or_default(),
        recipient,
        ..Default::default()
    }
}

pub fn exact_standing_order(
    asset_in: Address,
    asset_out: Address,
    exact_in: bool,
    amount: u128,
    min_price: U256,
    deadline: Option<u64>,
    recipient: Address
) -> ExactStandingOrder {
    ExactStandingOrder {
        asset_in,
        asset_out,
        min_price,
        max_extra_fee_asset0: if exact_in {
            amount
        } else {
            Ray::from(min_price)
                .mul_quantity(U256::from(amount))
                .saturating_to::<u128>()
        },
        deadline: deadline.map(|d| U40::from(d)).unwrap_or_default(),
        exact_in,
        amount,
        recipient,
        ..Default::default()
    }
}

pub fn partial_flash_order(
    asset_in: Address,
    asset_out: Address,
    min_amount_in: u128,
    max_amount_in: u128,
    min_price: U256,
    valid_for_block: u64,
    recipient: Address
) -> PartialFlashOrder {
    PartialFlashOrder {
        asset_in,
        asset_out,
        max_amount_in,
        min_amount_in,
        min_price,
        max_extra_fee_asset0: max_amount_in,
        valid_for_block,
        recipient,
        ..Default::default()
    }
}

pub fn exact_flash_order(
    asset_in: Address,
    asset_out: Address,
    exact_in: bool,
    amount: u128,
    min_price: U256,
    valid_for_block: u64,
    recipient: Address
) -> ExactFlashOrder {
    ExactFlashOrder {
        asset_in,
        asset_out,
        min_price,
        max_extra_fee_asset0: if exact_in {
            amount
        } else {
            Ray::from(min_price)
                .mul_quantity(U256::from(amount))
                .saturating_to::<u128>()
        },
        exact_in,
        amount,
        valid_for_block,
        recipient,
        ..Default::default()
    }
}

pub fn angstrom_eip712_domain(chain_id: u64) -> Eip712Domain {
    let mut domain = ANGSTROM_DOMAIN;
    domain.chain_id = Some(U256::from(chain_id));
    domain.chain_id = Some(U256::from(chain_id));
    domain
}
