//! Position Manager Liquidity Builder
//!
//! This module provides a type-safe builder pattern for constructing liquidity
//! management operations on Uniswap V4 pools. It supports the following
//! operations:
//! - Minting new positions
//! - Increasing liquidity in existing positions
//! - Decreasing liquidity from positions
//! - Burning positions
//! - Settlement and sweep operations
//!
//! # Example
//!
//! ```rust,ignore
//! use position_manager_liquidity::*;
//!
//! // Create a new liquidity manager
//! let mut manager = PositionManagerLiquidity::new();
//!
//! // Build a mint position operation with settlement
//! let mint_params = _liquidity_calls::mintPositionCall { /* ... */ };
//! let mut builder = PositionManagerLiquidityBuilder::<MintPosition>::new(mint_params)
//!     .mint_position();
//!
//! let settle_params = _liquidity_calls::settlePairCall { /* ... */ };
//! builder.add_settle(settle_params);
//!
//! // Chain the builder to the manager
//! manager.chain_builder(builder);
//! ```

use std::marker::PhantomData;

pub use _liquidity_calls::PositionManagerLiquidity;
use alloy_primitives::Bytes;
use alloy_sol_types::SolCall;

impl PositionManagerLiquidity {
    /// Creates a new empty PositionManagerLiquidity instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Chains a builder's actions and parameters into this
    /// PositionManagerLiquidity instance
    ///
    /// # Panics
    ///
    /// Panics if the builder's action count doesn't match its expected length
    pub fn chain_builder<M, A, B>(&mut self, builder: PositionManagerLiquidityBuilder<M, A, B>) {
        assert_eq!(builder.actions.len(), builder.assert_length);

        for (action, param) in builder.actions {
            self.actions = Bytes::from_iter(
                std::mem::take(&mut self.actions)
                    .into_iter()
                    .chain([action])
            );
            self.params.push(param.into());
        }
    }
}

/// Trait for handling liquidity actions with associated parameters and action
/// byte
pub trait HandleLiquidityAction {
    /// The Solidity call parameters type for this action
    type Params: SolCall;
    /// The unique action byte identifier for this operation
    const ACTION_BYTE: u8;
}

/// Action for increasing liquidity in an existing position
#[derive(Debug, Clone, Copy)]
pub struct IncreaseLiquidity;

impl HandleLiquidityAction for IncreaseLiquidity {
    type Params = _liquidity_calls::increaseLiquidityCall;

    const ACTION_BYTE: u8 = 0x00;
}

/// Action for decreasing liquidity from a position
#[derive(Debug, Clone, Copy)]
pub struct DescreaseLiquidity;

impl HandleLiquidityAction for DescreaseLiquidity {
    type Params = _liquidity_calls::decreaseLiquidityCall;

    const ACTION_BYTE: u8 = 0x01;
}

/// Action for minting a new liquidity position
#[derive(Debug, Clone, Copy)]
pub struct MintPosition;

impl HandleLiquidityAction for MintPosition {
    type Params = _liquidity_calls::mintPositionCall;

    const ACTION_BYTE: u8 = 0x02;
}

/// Action for burning a liquidity position
#[derive(Debug, Clone, Copy)]
pub struct BurnPosition;

impl HandleLiquidityAction for BurnPosition {
    type Params = _liquidity_calls::burnPositionCall;

    const ACTION_BYTE: u8 = 0x03;
}

/// Action for settling a currency pair (typically used after adding liquidity)
#[derive(Debug, Clone, Copy)]
pub struct SettlePair;

impl HandleLiquidityAction for SettlePair {
    type Params = _liquidity_calls::settlePairCall;

    const ACTION_BYTE: u8 = 0x0b;
}

/// Action for sweeping tokens to a recipient (typically used for ETH refunds)
#[derive(Debug, Clone, Copy)]
pub struct Sweep;

impl HandleLiquidityAction for Sweep {
    type Params = _liquidity_calls::sweepCall;

    const ACTION_BYTE: u8 = 0x14;
}

/// Action for taking tokens from a currency pair (typically used after removing
/// liquidity)
#[derive(Debug, Clone, Copy)]
pub struct TakePair;

impl HandleLiquidityAction for TakePair {
    type Params = _liquidity_calls::takePairCall;

    const ACTION_BYTE: u8 = 0x11;
}

/// Builder for constructing liquidity management operations with compile-time
/// type safety
///
/// The type parameters M, A, and B track the expected action sequence to ensure
/// operations are added in the correct order.
pub struct PositionManagerLiquidityBuilder<M, A = (), B = ()> {
    actions:       Vec<(u8, Vec<u8>)>,
    assert_length: usize,
    _phantom:      PhantomData<(M, A, B)>
}

impl<M: HandleLiquidityAction> PositionManagerLiquidityBuilder<M> {
    /// Creates a new builder with the initial action
    pub fn new(param: M::Params) -> Self {
        Self {
            actions:       vec![(M::ACTION_BYTE, param.abi_encode())],
            assert_length: 0,
            _phantom:      PhantomData
        }
    }
}

impl<M, A, B> PositionManagerLiquidityBuilder<M, A, B> {
    /// Internal method to add an action with length validation
    fn add_action<C: HandleLiquidityAction>(&mut self, param: C::Params, expected_length: usize) {
        self.actions.push((C::ACTION_BYTE, param.abi_encode()));
        assert_eq!(self.actions.len(), expected_length);
    }
}

impl PositionManagerLiquidityBuilder<IncreaseLiquidity> {
    /// Prepares to increase liquidity using ERC20 tokens
    ///
    /// This method sets up the builder to expect a SettlePair action
    /// for settling the token transfers
    pub fn increase_liquidity_with_erc20(
        self
    ) -> PositionManagerLiquidityBuilder<IncreaseLiquidity, SettlePair> {
        PositionManagerLiquidityBuilder {
            actions:       self.actions,
            assert_length: 2,
            _phantom:      PhantomData
        }
    }

    /// Prepares to increase liquidity using ETH
    ///
    /// This method sets up the builder to expect both SettlePair and Sweep
    /// actions for settling tokens and refunding excess ETH
    pub fn increase_liquidity_with_eth(
        self
    ) -> PositionManagerLiquidityBuilder<IncreaseLiquidity, SettlePair, Sweep> {
        PositionManagerLiquidityBuilder {
            actions:       self.actions,
            assert_length: 3,
            _phantom:      PhantomData
        }
    }
}

impl PositionManagerLiquidityBuilder<MintPosition> {
    /// Prepares to mint a new position
    ///
    /// This method sets up the builder to expect a SettlePair action
    /// for settling the initial liquidity tokens
    pub fn mint_position(self) -> PositionManagerLiquidityBuilder<MintPosition, SettlePair> {
        PositionManagerLiquidityBuilder {
            actions:       self.actions,
            assert_length: 2,
            _phantom:      PhantomData
        }
    }
}

impl PositionManagerLiquidityBuilder<DescreaseLiquidity> {
    /// Prepares to decrease liquidity before burning the position
    ///
    /// This method transitions from BurnPosition to DecreaseLiquidity action
    /// type
    pub fn decrease_liquidity(
        self
    ) -> PositionManagerLiquidityBuilder<DescreaseLiquidity, TakePair> {
        PositionManagerLiquidityBuilder {
            actions:       self.actions,
            assert_length: 2,
            _phantom:      PhantomData
        }
    }
}

impl PositionManagerLiquidityBuilder<BurnPosition> {
    /// Prepares to burn a position
    ///
    /// This method sets up the builder to expect a TakePair action
    /// for withdrawing the tokens after burning
    pub fn burn_position(self) -> PositionManagerLiquidityBuilder<BurnPosition, TakePair> {
        PositionManagerLiquidityBuilder {
            actions:       self.actions,
            assert_length: 2,
            _phantom:      PhantomData
        }
    }
}

impl<M, B> PositionManagerLiquidityBuilder<M, SettlePair, B> {
    /// Adds a settle pair action to the builder
    ///
    /// This settles the token transfers after adding liquidity
    pub fn add_settle(&mut self, settle: <SettlePair as HandleLiquidityAction>::Params) {
        self.add_action::<SettlePair>(settle, 2);
    }
}

impl<M> PositionManagerLiquidityBuilder<M, SettlePair, Sweep> {
    /// Adds a sweep action to the builder
    ///
    /// This refunds excess ETH after liquidity operations
    pub fn add_sweep(&mut self, sweep: <Sweep as HandleLiquidityAction>::Params) {
        self.add_action::<Sweep>(sweep, 3);
    }
}

impl<M> PositionManagerLiquidityBuilder<M, TakePair> {
    /// Adds a take pair action to the builder
    ///
    /// This withdraws tokens after removing liquidity
    pub fn add_take_pair(&mut self, take: <TakePair as HandleLiquidityAction>::Params) {
        self.add_action::<TakePair>(take, 2);
    }
}

/// Module containing Solidity function call definitions and types
pub mod _liquidity_calls {
    use alloy::sol;
    use angstrom_types::contract_bindings::{
        pool_manager::PoolManager, position_manager::PositionManager
    };

    impl From<PoolManager::PoolKey> for PoolKey {
        fn from(value: PoolManager::PoolKey) -> Self {
            PoolKey {
                currency0:   value.currency0,
                currency1:   value.currency1,
                fee:         value.fee,
                tickSpacing: value.tickSpacing,
                hooks:       value.hooks
            }
        }
    }

    impl From<PositionManager::PoolKey> for PoolKey {
        fn from(value: PositionManager::PoolKey) -> Self {
            PoolKey {
                currency0:   value.currency0,
                currency1:   value.currency1,
                fee:         value.fee,
                tickSpacing: value.tickSpacing,
                hooks:       value.hooks
            }
        }
    }

    impl Copy for PoolKey {}

    sol! {

        #[derive(Debug, Default)]
        struct PositionManagerLiquidity {
            bytes actions;
            bytes[] params;
        }

        struct PoolKey {
            address currency0;
            address currency1;
            uint24 fee;
            int24 tickSpacing;
            address hooks;
        }

        function increaseLiquidity(
            uint256 tokenId,
            uint256 liquidity,
            uint128 amount0Max,
            uint128 amount1Max,
            bytes calldata hookData
        );

        function decreaseLiquidity(
            uint256 tokenId,
            uint256 liquidity,
            uint128 amount0Min,
            uint128 amount1Min,
            bytes calldata hookData
        );
        function mintPosition(
            PoolKey calldata poolKey,
            int24 tickLower,
            int24 tickUpper,
            uint256 liquidity,
            uint128 amount0Max,
            uint128 amount1Max,
            address owner,
            bytes calldata hookData
        );

        function burnPosition(
            uint256 tokenId,
            uint128 amount0Min,
            uint128 amount1Min,
            bytes calldata hookData
        );

        function takePair(
            address currency0,
            address currency1,
            address recipient,
        );

        function settlePair(
            address currency0,
            address currency1
        );

        function sweep(
            address currency,
            address recipient,
        );


    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{
        Address, Bytes, U256, address,
        aliases::{I24, U24}
    };
    use alloy_sol_types::SolCall;

    use super::*;

    fn create_test_pool_key() -> _liquidity_calls::PoolKey {
        _liquidity_calls::PoolKey {
            currency0:   address!("1111111111111111111111111111111111111111"),
            currency1:   address!("2222222222222222222222222222222222222222"),
            fee:         U24::from(500),
            tickSpacing: I24::try_from(10).unwrap(),
            hooks:       Address::ZERO
        }
    }

    #[test]
    fn test_position_manager_liquidity_new() {
        let liquidity_manager = PositionManagerLiquidity::new();
        assert_eq!(liquidity_manager.actions, Bytes::default());
        assert_eq!(liquidity_manager.params.len(), 0);
    }

    #[test]
    fn test_increase_liquidity_action_byte() {
        assert_eq!(IncreaseLiquidity::ACTION_BYTE, 0x00);
    }

    #[test]
    fn test_decrease_liquidity_action_byte() {
        assert_eq!(DescreaseLiquidity::ACTION_BYTE, 0x01);
    }

    #[test]
    fn test_mint_position_action_byte() {
        assert_eq!(MintPosition::ACTION_BYTE, 0x02);
    }

    #[test]
    fn test_burn_position_action_byte() {
        assert_eq!(BurnPosition::ACTION_BYTE, 0x03);
    }

    #[test]
    fn test_settle_pair_action_byte() {
        assert_eq!(SettlePair::ACTION_BYTE, 0x0b);
    }

    #[test]
    fn test_sweep_action_byte() {
        assert_eq!(Sweep::ACTION_BYTE, 0x14);
    }

    #[test]
    fn test_take_pair_action_byte() {
        assert_eq!(TakePair::ACTION_BYTE, 0x11);
    }

    #[test]
    fn test_increase_liquidity_builder() {
        let params = _liquidity_calls::increaseLiquidityCall {
            tokenId:    U256::from(123),
            liquidity:  U256::from(1000),
            amount0Max: 5000,
            amount1Max: 5000,
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<IncreaseLiquidity>::new(params.clone());
        assert_eq!(builder.actions.len(), 1);
        assert_eq!(builder.actions[0].0, IncreaseLiquidity::ACTION_BYTE);
        assert_eq!(builder.actions[0].1, params.abi_encode());
    }

    #[test]
    fn test_increase_liquidity_with_erc20() {
        let params = _liquidity_calls::increaseLiquidityCall {
            tokenId:    U256::from(123),
            liquidity:  U256::from(1000),
            amount0Max: 5000,
            amount1Max: 5000,
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<IncreaseLiquidity>::new(params);
        let mut builder_with_settle = builder.increase_liquidity_with_erc20();

        assert_eq!(builder_with_settle.assert_length, 2);

        let settle_params = _liquidity_calls::settlePairCall {
            currency0: address!("1111111111111111111111111111111111111111"),
            currency1: address!("2222222222222222222222222222222222222222")
        };

        builder_with_settle.add_settle(settle_params);
        assert_eq!(builder_with_settle.actions.len(), 2);
        assert_eq!(builder_with_settle.actions[1].0, SettlePair::ACTION_BYTE);
    }

    #[test]
    fn test_increase_liquidity_with_eth() {
        let params = _liquidity_calls::increaseLiquidityCall {
            tokenId:    U256::from(123),
            liquidity:  U256::from(1000),
            amount0Max: 5000,
            amount1Max: 5000,
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<IncreaseLiquidity>::new(params);
        let mut builder_with_eth = builder.increase_liquidity_with_eth();

        assert_eq!(builder_with_eth.assert_length, 3);

        let settle_params = _liquidity_calls::settlePairCall {
            currency0: address!("1111111111111111111111111111111111111111"),
            currency1: address!("2222222222222222222222222222222222222222")
        };

        builder_with_eth.add_settle(settle_params);
        assert_eq!(builder_with_eth.actions.len(), 2);

        let sweep_params = _liquidity_calls::sweepCall {
            currency:  Address::ZERO,
            recipient: address!("3333333333333333333333333333333333333333")
        };

        builder_with_eth.add_sweep(sweep_params);
        assert_eq!(builder_with_eth.actions.len(), 3);
        assert_eq!(builder_with_eth.actions[2].0, Sweep::ACTION_BYTE);
    }

    #[test]
    fn test_mint_position_builder() {
        let pool_key = create_test_pool_key();
        let params = _liquidity_calls::mintPositionCall {
            poolKey:    pool_key,
            tickLower:  I24::try_from(-100).unwrap(),
            tickUpper:  I24::try_from(100).unwrap(),
            liquidity:  U256::from(10000),
            amount0Max: 50000,
            amount1Max: 50000,
            owner:      address!("4444444444444444444444444444444444444444"),
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<MintPosition>::new(params.clone());
        assert_eq!(builder.actions.len(), 1);
        assert_eq!(builder.actions[0].0, MintPosition::ACTION_BYTE);
        assert_eq!(builder.actions[0].1, params.abi_encode());
    }

    #[test]
    fn test_mint_position_with_settle() {
        let pool_key = create_test_pool_key();
        let params = _liquidity_calls::mintPositionCall {
            poolKey:    pool_key,
            tickLower:  I24::try_from(-100).unwrap(),
            tickUpper:  I24::try_from(100).unwrap(),
            liquidity:  U256::from(10000),
            amount0Max: 50000,
            amount1Max: 50000,
            owner:      address!("4444444444444444444444444444444444444444"),
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<MintPosition>::new(params);
        let mut builder_with_settle = builder.mint_position();

        assert_eq!(builder_with_settle.assert_length, 2);

        let settle_params = _liquidity_calls::settlePairCall {
            currency0: address!("1111111111111111111111111111111111111111"),
            currency1: address!("2222222222222222222222222222222222222222")
        };

        builder_with_settle.add_settle(settle_params);
        assert_eq!(builder_with_settle.actions.len(), 2);
    }

    #[test]
    fn test_burn_position_builder() {
        let params = _liquidity_calls::burnPositionCall {
            tokenId:    U256::from(456),
            amount0Min: 1000,
            amount1Min: 1000,
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<BurnPosition>::new(params.clone());
        assert_eq!(builder.actions.len(), 1);
        assert_eq!(builder.actions[0].0, BurnPosition::ACTION_BYTE);
        assert_eq!(builder.actions[0].1, params.abi_encode());
    }

    #[test]
    fn test_burn_position_with_take_pair() {
        let params = _liquidity_calls::burnPositionCall {
            tokenId:    U256::from(456),
            amount0Min: 1000,
            amount1Min: 1000,
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<BurnPosition>::new(params);
        let mut builder_with_take = builder.burn_position();

        assert_eq!(builder_with_take.assert_length, 2);

        let take_params = _liquidity_calls::takePairCall {
            currency0: address!("1111111111111111111111111111111111111111"),
            currency1: address!("2222222222222222222222222222222222222222"),
            recipient: address!("5555555555555555555555555555555555555555")
        };

        builder_with_take.add_take_pair(take_params);
        assert_eq!(builder_with_take.actions.len(), 2);
        assert_eq!(builder_with_take.actions[1].0, TakePair::ACTION_BYTE);
    }

    #[test]
    fn test_decrease_liquidity_builder() {
        let decrease_liquidity_params = _liquidity_calls::decreaseLiquidityCall {
            tokenId:    U256::from(789),
            amount0Min: 2000,
            amount1Min: 2000,
            hookData:   Bytes::default(),
            liquidity:  U256::from(12995849)
        };

        let builder =
            PositionManagerLiquidityBuilder::<DescreaseLiquidity>::new(decrease_liquidity_params);
        let builder_with_decrease = builder.decrease_liquidity();

        assert_eq!(builder_with_decrease.assert_length, 2);
    }

    #[test]
    fn test_chain_builder() {
        let mut liquidity_manager = PositionManagerLiquidity::new();

        let increase_params = _liquidity_calls::increaseLiquidityCall {
            tokenId:    U256::from(123),
            liquidity:  U256::from(1000),
            amount0Max: 5000,
            amount1Max: 5000,
            hookData:   Bytes::default()
        };

        let mut builder =
            PositionManagerLiquidityBuilder::<IncreaseLiquidity>::new(increase_params)
                .increase_liquidity_with_erc20();

        let settle_params = _liquidity_calls::settlePairCall {
            currency0: address!("1111111111111111111111111111111111111111"),
            currency1: address!("2222222222222222222222222222222222222222")
        };

        builder.add_settle(settle_params);

        liquidity_manager.chain_builder(builder);

        assert_eq!(liquidity_manager.actions.len(), 2);
        assert_eq!(liquidity_manager.params.len(), 2);
        assert_eq!(liquidity_manager.actions[0], IncreaseLiquidity::ACTION_BYTE);
        assert_eq!(liquidity_manager.actions[1], SettlePair::ACTION_BYTE);
    }

    #[test]
    fn test_complete_mint_flow() {
        let mut liquidity_manager = PositionManagerLiquidity::new();

        let pool_key = create_test_pool_key();
        let mint_params = _liquidity_calls::mintPositionCall {
            poolKey:    pool_key,
            tickLower:  I24::try_from(-1000).unwrap(),
            tickUpper:  I24::try_from(1000).unwrap(),
            liquidity:  U256::from(100000),
            amount0Max: 500000,
            amount1Max: 500000,
            owner:      address!("6666666666666666666666666666666666666666"),
            hookData:   Bytes::default()
        };

        let mut builder =
            PositionManagerLiquidityBuilder::<MintPosition>::new(mint_params).mint_position();

        let settle_params = _liquidity_calls::settlePairCall {
            currency0: pool_key.currency0,
            currency1: pool_key.currency1
        };

        builder.add_settle(settle_params);

        liquidity_manager.chain_builder(builder);

        assert_eq!(liquidity_manager.actions.len(), 2);
        assert_eq!(liquidity_manager.params.len(), 2);
    }

    #[test]
    fn test_complete_burn_flow() {
        let mut liquidity_manager = PositionManagerLiquidity::new();

        let burn_params = _liquidity_calls::burnPositionCall {
            tokenId:    U256::from(999),
            amount0Min: 10000,
            amount1Min: 10000,
            hookData:   Bytes::default()
        };

        let mut builder =
            PositionManagerLiquidityBuilder::<BurnPosition>::new(burn_params).burn_position();

        let take_params = _liquidity_calls::takePairCall {
            currency0: address!("1111111111111111111111111111111111111111"),
            currency1: address!("2222222222222222222222222222222222222222"),
            recipient: address!("7777777777777777777777777777777777777777")
        };

        builder.add_take_pair(take_params);

        liquidity_manager.chain_builder(builder);

        assert_eq!(liquidity_manager.actions.len(), 2);
        assert_eq!(liquidity_manager.params.len(), 2);
        assert_eq!(liquidity_manager.actions[0], BurnPosition::ACTION_BYTE);
        assert_eq!(liquidity_manager.actions[1], TakePair::ACTION_BYTE);
    }

    #[test]
    #[should_panic]
    fn test_chain_builder_assert_mismatch() {
        let mut liquidity_manager = PositionManagerLiquidity::new();

        let increase_params = _liquidity_calls::increaseLiquidityCall {
            tokenId:    U256::from(123),
            liquidity:  U256::from(1000),
            amount0Max: 5000,
            amount1Max: 5000,
            hookData:   Bytes::default()
        };

        let builder = PositionManagerLiquidityBuilder::<IncreaseLiquidity>::new(increase_params)
            .increase_liquidity_with_erc20();

        liquidity_manager.chain_builder(builder);
    }

    #[test]
    fn test_pool_key_conversion_from_pool_manager() {
        use angstrom_types::contract_bindings::pool_manager::PoolManager;

        let pm_key = PoolManager::PoolKey {
            currency0:   address!("1111111111111111111111111111111111111111"),
            currency1:   address!("2222222222222222222222222222222222222222"),
            fee:         U24::from(3000),
            tickSpacing: I24::try_from(60).unwrap(),
            hooks:       address!("8888888888888888888888888888888888888888")
        };

        let converted_key: _liquidity_calls::PoolKey = pm_key.into();

        assert_eq!(converted_key.currency0, pm_key.currency0);
        assert_eq!(converted_key.currency1, pm_key.currency1);
        assert_eq!(converted_key.fee, pm_key.fee);
        assert_eq!(converted_key.tickSpacing, pm_key.tickSpacing);
        assert_eq!(converted_key.hooks, pm_key.hooks);
    }

    #[test]
    fn test_pool_key_conversion_from_position_manager() {
        use angstrom_types::contract_bindings::position_manager::PositionManager;

        let pm_key = PositionManager::PoolKey {
            currency0:   address!("1111111111111111111111111111111111111111"),
            currency1:   address!("2222222222222222222222222222222222222222"),
            fee:         U24::from(10000),
            tickSpacing: I24::try_from(200).unwrap(),
            hooks:       Address::ZERO
        };

        let converted_key: _liquidity_calls::PoolKey = pm_key.into();

        assert_eq!(converted_key.currency0, pm_key.currency0);
        assert_eq!(converted_key.currency1, pm_key.currency1);
        assert_eq!(converted_key.fee, pm_key.fee);
        assert_eq!(converted_key.tickSpacing, pm_key.tickSpacing);
        assert_eq!(converted_key.hooks, pm_key.hooks);
    }
}
