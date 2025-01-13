use std::collections::HashMap;

use alloy_primitives::{
    aliases::{I24, U24},
    Address, Bytes, B256, U256
};
use angstrom_sdk_macros::{neon_object_as, NeonObject};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::{
        angstrom::{OrderQuantities, StandingValidation, TopOfBlockOrder, UserOrder},
        Signature
    },
    primitive::{PoolId, UniswapPoolRegistry}
};
use neon::object::Object;
use uniswap_v4::uniswap::{
    pool::{EnhancedUniswapPool, TickInfo},
    pool_data_loader::{DataLoader, PoolDataLoader}
};

use crate::types::HistoricalOrders;

#[derive(Debug, Clone, NeonObject)]
pub struct PoolKeyNeon {
    currency0:   Address,
    currency1:   Address,
    fee:         U24,
    tickSpacing: I24,
    hooks:       Address
}

impl From<PoolKey> for PoolKeyNeon {
    fn from(value: PoolKey) -> Self {
        Self {
            currency0:   value.currency0,
            currency1:   value.currency1,
            fee:         value.fee,
            tickSpacing: value.tickSpacing,
            hooks:       value.hooks
        }
    }
}

impl Into<PoolKey> for PoolKeyNeon {
    fn into(self) -> PoolKey {
        PoolKey {
            currency0:   self.currency0,
            currency1:   self.currency1,
            fee:         self.fee,
            tickSpacing: self.tickSpacing,
            hooks:       self.hooks
        }
    }
}

neon_object_as!(PoolKey, PoolKeyNeon);

#[derive(Debug, Clone, NeonObject)]
pub enum HistoricalOrdersNeon {
    TOB { order: TopOfBlockOrderNeon },
    User { order: UserOrderNeon }
}

impl From<HistoricalOrders> for HistoricalOrdersNeon {
    fn from(value: HistoricalOrders) -> Self {
        match value {
            HistoricalOrders::TOB(top_of_block_order) => {
                Self::TOB { order: top_of_block_order.into() }
            }
            HistoricalOrders::User(user_order) => Self::User { order: user_order.into() }
        }
    }
}

impl Into<HistoricalOrders> for HistoricalOrdersNeon {
    fn into(self) -> HistoricalOrders {
        match self {
            Self::TOB { order } => HistoricalOrders::TOB(order.into()),
            Self::User { order } => HistoricalOrders::User(order.into())
        }
    }
}

neon_object_as!(HistoricalOrders, HistoricalOrdersNeon);

#[derive(Debug, Clone, NeonObject)]
pub struct TopOfBlockOrderNeon {
    use_internal:     bool,
    quantity_in:      u128,
    quantity_out:     u128,
    max_gas_asset_0:  u128,
    gas_used_asset_0: u128,
    pairs_index:      u16,
    zero_for_1:       bool,
    recipient:        Option<Address>,
    signature:        SignatureNeon
}

impl From<TopOfBlockOrder> for TopOfBlockOrderNeon {
    fn from(value: TopOfBlockOrder) -> Self {
        Self {
            use_internal:     value.use_internal,
            quantity_in:      value.quantity_in,
            quantity_out:     value.quantity_out,
            max_gas_asset_0:  value.max_gas_asset_0,
            gas_used_asset_0: value.gas_used_asset_0,
            pairs_index:      value.pairs_index,
            zero_for_1:       value.zero_for_1,
            recipient:        value.recipient,
            signature:        value.signature.into()
        }
    }
}

impl Into<TopOfBlockOrder> for TopOfBlockOrderNeon {
    fn into(self) -> TopOfBlockOrder {
        TopOfBlockOrder {
            use_internal:     self.use_internal,
            quantity_in:      self.quantity_in,
            quantity_out:     self.quantity_out,
            max_gas_asset_0:  self.max_gas_asset_0,
            gas_used_asset_0: self.gas_used_asset_0,
            pairs_index:      self.pairs_index,
            zero_for_1:       self.zero_for_1,
            recipient:        self.recipient,
            signature:        self.signature.into()
        }
    }
}

neon_object_as!(TopOfBlockOrder, TopOfBlockOrderNeon);

#[derive(Debug, Clone, NeonObject)]
pub struct UserOrderNeon {
    ref_id:               u32,
    use_internal:         bool,
    pair_index:           u16,
    min_price:            U256,
    recipient:            Option<Address>,
    hook_data:            Option<Bytes>,
    zero_for_one:         bool,
    standing_validation:  Option<StandingValidationNeon>,
    order_quantities:     OrderQuantitiesNeon,
    max_extra_fee_asset0: u128,
    extra_fee_asset0:     u128,
    exact_in:             bool,
    signature:            SignatureNeon
}

impl From<UserOrder> for UserOrderNeon {
    fn from(value: UserOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            recipient:            value.recipient,
            signature:            value.signature.into(),
            use_internal:         value.use_internal,
            pair_index:           value.pair_index,
            min_price:            value.min_price,
            hook_data:            value.hook_data,
            zero_for_one:         value.zero_for_one,
            standing_validation:  value.standing_validation.map(Into::into),
            order_quantities:     value.order_quantities.into(),
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            extra_fee_asset0:     value.extra_fee_asset0,
            exact_in:             value.exact_in
        }
    }
}

impl Into<UserOrder> for UserOrderNeon {
    fn into(self) -> UserOrder {
        UserOrder {
            ref_id:               self.ref_id,
            recipient:            self.recipient,
            signature:            self.signature.into(),
            use_internal:         self.use_internal,
            pair_index:           self.pair_index,
            min_price:            self.min_price,
            hook_data:            self.hook_data,
            zero_for_one:         self.zero_for_one,
            standing_validation:  self.standing_validation.map(Into::into),
            order_quantities:     self.order_quantities.into(),
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            extra_fee_asset0:     self.extra_fee_asset0,
            exact_in:             self.exact_in
        }
    }
}

neon_object_as!(UserOrder, UserOrderNeon);

#[derive(Debug, Clone, NeonObject)]
enum SignatureNeon {
    Contract { from: Address, signature: Bytes },
    Ecdsa { v: u8, r: B256, s: B256 }
}

impl From<Signature> for SignatureNeon {
    fn from(value: Signature) -> Self {
        match value {
            Signature::Contract { from, signature } => Self::Contract { from, signature },
            Signature::Ecdsa { v, r, s } => Self::Ecdsa { v, r, s }
        }
    }
}

impl Into<Signature> for SignatureNeon {
    fn into(self) -> Signature {
        match self {
            Self::Contract { from, signature } => Signature::Contract { from, signature },
            Self::Ecdsa { v, r, s } => Signature::Ecdsa { v, r, s }
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
enum OrderQuantitiesNeon {
    Exact { quantity: u128 },
    Partial { min_quantity_in: u128, max_quantity_in: u128, filled_quantity: u128 }
}

impl From<OrderQuantities> for OrderQuantitiesNeon {
    fn from(value: OrderQuantities) -> Self {
        match value {
            OrderQuantities::Exact { quantity } => Self::Exact { quantity },
            OrderQuantities::Partial { min_quantity_in, max_quantity_in, filled_quantity } => {
                Self::Partial { min_quantity_in, max_quantity_in, filled_quantity }
            }
        }
    }
}

impl Into<OrderQuantities> for OrderQuantitiesNeon {
    fn into(self) -> OrderQuantities {
        match self {
            Self::Exact { quantity } => OrderQuantities::Exact { quantity },
            Self::Partial { min_quantity_in, max_quantity_in, filled_quantity } => {
                OrderQuantities::Partial { min_quantity_in, max_quantity_in, filled_quantity }
            }
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
struct StandingValidationNeon {
    nonce:    u64,
    deadline: u64
}

impl From<StandingValidation> for StandingValidationNeon {
    fn from(value: StandingValidation) -> Self {
        StandingValidationNeon { nonce: value.nonce(), deadline: value.deadline() }
    }
}

impl Into<StandingValidation> for StandingValidationNeon {
    fn into(self) -> StandingValidation {
        StandingValidation::new(self.nonce, self.deadline)
    }
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct EnhancedUniswapPoolNeon {
    sync_swap_with_sim:     bool,
    initial_ticks_per_side: u16,
    data_loader:            DataLoaderNeon,
    token_a:                Address,
    token_a_decimals:       u8,
    token_b:                Address,
    token_b_decimals:       u8,
    liquidity:              u128,
    liquidity_net:          i128,
    sqrt_price:             U256,
    fee:                    u32,
    tick:                   i32,
    tick_spacing:           i32,
    tick_bitmap:            HashMap<i16, U256>,
    ticks:                  HashMap<i32, TickInfoNeon>
}

impl From<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>> for EnhancedUniswapPoolNeon {
    fn from(value: EnhancedUniswapPool<DataLoader<PoolId>, PoolId>) -> Self {
        EnhancedUniswapPoolNeon {
            sync_swap_with_sim:     value.is_sync_swap_with_sim(),
            initial_ticks_per_side: value.initial_ticks_per_side(),
            data_loader:            value.data_loader.into(),
            token_a:                value.token_a,
            token_a_decimals:       value.token_a_decimals,
            token_b:                value.token_b,
            token_b_decimals:       value.token_b_decimals,
            liquidity:              value.liquidity,
            liquidity_net:          value.liquidity_net,
            sqrt_price:             value.sqrt_price,
            fee:                    value.fee,
            tick:                   value.tick,
            tick_spacing:           value.tick_spacing,
            tick_bitmap:            value.tick_bitmap,
            ticks:                  value
                .ticks
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect()
        }
    }
}

impl Into<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>> for EnhancedUniswapPoolNeon {
    fn into(self) -> EnhancedUniswapPool<DataLoader<PoolId>, PoolId> {
        let mut pool =
            EnhancedUniswapPool::new(self.data_loader.into(), self.initial_ticks_per_side);
        pool.set_sim_swap_sync(self.sync_swap_with_sim);

        pool.token_a = self.token_a;
        pool.token_a_decimals = self.token_a_decimals;
        pool.token_b = self.token_b;
        pool.token_b_decimals = self.token_b_decimals;
        pool.liquidity = self.liquidity;
        pool.liquidity_net = self.liquidity_net;
        pool.sqrt_price = self.sqrt_price;
        pool.fee = self.fee;
        pool.tick = self.tick;
        pool.tick_spacing = self.tick_spacing;
        pool.tick_bitmap = self.tick_bitmap;
        pool.ticks = self.ticks.into_iter().map(|(k, v)| (k, v.into())).collect();

        pool
    }
}

neon_object_as!(EnhancedUniswapPool<DataLoader<PoolId>, PoolId>, EnhancedUniswapPoolNeon);

#[derive(Clone, Default)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct DataLoaderNeon {
    address_or_pool_id: B256,
    pool_registry:      Option<UniswapPoolRegistryNeon>,
    pool_manager:       Option<Address>
}

impl From<DataLoader<PoolId>> for DataLoaderNeon {
    fn from(value: DataLoader<PoolId>) -> Self {
        DataLoaderNeon {
            address_or_pool_id: value.address(),
            pool_manager:       value.pool_manager_opt(),
            pool_registry:      value.pool_registry().map(Into::into)
        }
    }
}

impl Into<DataLoader<PoolId>> for DataLoaderNeon {
    fn into(self) -> DataLoader<PoolId> {
        if let Some((reg, man)) = self.pool_registry.zip(self.pool_manager) {
            DataLoader::new_with_registry(self.address_or_pool_id, reg.into(), man)
        } else {
            panic!("pool_registry and pool_manager cannot be none");
        }
    }
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct UniswapPoolRegistryNeon {
    pools: HashMap<B256, PoolKeyNeon>
}

impl From<UniswapPoolRegistry> for UniswapPoolRegistryNeon {
    fn from(value: UniswapPoolRegistry) -> Self {
        UniswapPoolRegistryNeon {
            pools: value
                .pools()
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect()
        }
    }
}

impl Into<UniswapPoolRegistry> for UniswapPoolRegistryNeon {
    fn into(self) -> UniswapPoolRegistry {
        UniswapPoolRegistry::from(
            self.pools
                .into_iter()
                .map(|(_, v)| v.into())
                .collect::<Vec<_>>()
        )
    }
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct TickInfoNeon {
    liquidity_gross: u128,
    liquidity_net:   i128,
    initialized:     bool
}

impl From<TickInfo> for TickInfoNeon {
    fn from(value: TickInfo) -> Self {
        TickInfoNeon {
            liquidity_gross: value.liquidity_gross,
            liquidity_net:   value.liquidity_net,
            initialized:     value.initialized
        }
    }
}

impl Into<TickInfo> for TickInfoNeon {
    fn into(self) -> TickInfo {
        TickInfo {
            liquidity_gross: self.liquidity_gross,
            liquidity_net:   self.liquidity_net,
            initialized:     self.initialized
        }
    }
}
