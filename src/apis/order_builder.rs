use alloy_primitives::{
    Address, Bytes, I256,
    aliases::{I24, U24}
};
use alloy_sol_types::SolCall;
use angstrom_types::{
    contract_bindings::pool_manager::{IPoolManager, PoolManager},
    primitive::ANGSTROM_ADDRESS,
    sol_bindings::{grouped_orders::AllOrders, rpc_orders::TopOfBlockOrder}
};
use testing_tools::type_generator::orders::{ToBOrderBuilder, UserOrderBuilder};

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

    pub fn modify_liquidity(
        token0: Address,
        token1: Address,
        tick_lower: i32,
        tick_upper: i32,
        pool_tick_spacing: i32,
        liquidity_delta: I256
    ) -> PoolManager::unlockCall {
        let params = IPoolManager::ModifyLiquidityParams {
            tickLower:      I24::unchecked_from(tick_lower),
            tickUpper:      I24::unchecked_from(tick_upper),
            liquidityDelta: liquidity_delta,
            salt:           Default::default()
        };

        let pool_key = PoolManager::PoolKey {
            currency0:   token0,
            currency1:   token1,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(pool_tick_spacing),
            hooks:       *ANGSTROM_ADDRESS.get().unwrap()
        };

        let modify_liq_call =
            PoolManager::modifyLiquidityCall { key: pool_key, params, hookData: Bytes::default() };

        PoolManager::unlockCall { data: modify_liq_call.abi_encode().into() }
    }
}

mod _liquidity_calls {
    use alloy::sol;

    sol! {
        function addLiquidity(
            address asset0,
            address asset1,
            int24 tickLower,
            int24 tickUpper,
            uint256 liquidity,
            bytes32 salt
        ) public returns (int256 callerDelta, int256 feesAccrued);

        function removeLiquidity(
            address asset0,
            address asset1,
            int24 tickLower,
            int24 tickUpper,
            uint256 liquidity,
            bytes32 salt
        ) public returns (int256 callerDelta, int256 feesAccrued);
    }
}
