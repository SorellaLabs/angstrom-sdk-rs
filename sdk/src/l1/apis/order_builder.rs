use alloy_primitives::{B256, Bytes, I256, U256, aliases::I24};
use alloy_sol_types::{SolCall, SolValue};
use angstrom_types_primitives::{
    contract_bindings::{
        pool_manager::{IPoolManager, PoolManager},
        position_manager::PositionManager
    },
    orders::builders::{ToBOrderBuilder, UserOrderBuilder},
    sol_bindings::{grouped_orders::AllOrders, rpc_orders::TopOfBlockOrder}
};

use crate::l1::builders::PositionManagerLiquidity;

pub struct AngstromOrderBuilder;

impl AngstromOrderBuilder {
    pub fn tob_order(f: impl Fn(ToBOrderBuilder) -> TopOfBlockOrder) -> AllOrders {
        AllOrders::TOB(f(ToBOrderBuilder::new()))
    }

    pub fn flash_order(f: impl Fn(UserOrderBuilder) -> AllOrders) -> AllOrders {
        f(UserOrderBuilder::new().kill_or_fill())
    }

    pub fn standing_order(f: impl Fn(UserOrderBuilder) -> AllOrders) -> AllOrders {
        f(UserOrderBuilder::new().standing())
    }

    /// through PoolManager
    pub fn modify_liquidity(
        pool_key: PoolManager::PoolKey,
        tick_lower: I24,
        tick_upper: I24,
        liquidity_delta: I256,
        hook_data: Bytes,
        salt: B256
    ) -> PoolManager::unlockCall {
        let params = IPoolManager::ModifyLiquidityParams {
            tickLower: tick_lower,
            tickUpper: tick_upper,
            liquidityDelta: liquidity_delta,
            salt
        };

        let modify_liq_call =
            PoolManager::modifyLiquidityCall { key: pool_key, params, hookData: hook_data };

        PoolManager::unlockCall { data: modify_liq_call.abi_encode().into() }
    }

    /// through PositionManager
    pub fn modify_liquidities(
        position_manager_liquidity: PositionManagerLiquidity,
        deadline: U256
    ) -> PositionManager::modifyLiquiditiesCall {
        PositionManager::modifyLiquiditiesCall {
            unlockData: position_manager_liquidity.abi_encode().into(),
            deadline
        }
    }
}
