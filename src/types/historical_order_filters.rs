use std::{
    collections::{HashMap, HashSet},
    str::FromStr
};

use alloy_consensus::Transaction;
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types::Block;
#[cfg(feature = "neon")]
use angstrom_sdk_macros::NeonObject;
use angstrom_types::{
    contract_payloads::angstrom::{AngstromBundle, TopOfBlockOrder, UserOrder},
    primitive::PoolId
};
#[cfg(feature = "neon")]
use neon::object::Object;
use pade::PadeDecode;

use super::PoolMetadata;
use crate::{apis::utils::pool_config_store, providers::EthRpcProvider};

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct HistoricalOrdersFilter {
    pub order_kinds:   HashSet<OrderKind>,
    pub order_filters: HashSet<OrderFilter>,
    pub from_block:    Option<u64>,
    pub to_block:      Option<u64>
}

impl HistoricalOrdersFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_block(mut self, block: u64) -> Self {
        self.from_block = Some(block);
        self
    }

    pub fn to_block(mut self, block: u64) -> Self {
        self.to_block = Some(block);
        self
    }

    pub fn order_filter(mut self, filter: OrderFilter) -> Self {
        self.order_filters.insert(filter);
        self
    }

    pub fn order_filters(mut self, filters: impl IntoIterator<Item = OrderFilter>) -> Self {
        self.order_filters.extend(filters);
        self
    }

    pub fn order_kind(mut self, order_kind: OrderKind) -> Self {
        self.order_kinds.insert(order_kind);
        self
    }

    pub fn order_kinds(mut self, order_kinds: impl IntoIterator<Item = OrderKind>) -> Self {
        self.order_kinds.extend(order_kinds);
        self
    }

    pub fn filter_block(
        &self,
        block: Block,
        pool_stores: &AngstromPoolTokenIndexToPair
    ) -> Vec<HistoricalOrders> {
        block
            .transactions
            .into_transactions()
            .filter_map(|transaction| {
                let mut input: &[u8] = transaction.input();
                AngstromBundle::pade_decode(&mut input, None).ok()
            })
            .flat_map(|bundle| self.apply_kinds(bundle, pool_stores))
            .collect()
    }

    fn apply_kinds(
        &self,
        bundle: AngstromBundle,
        pool_stores: &AngstromPoolTokenIndexToPair
    ) -> Vec<HistoricalOrders> {
        let mut all_orders = Vec::new();

        if self.order_kinds.contains(&OrderKind::TOB) || self.order_kinds.contains(&OrderKind::None)
        {
            bundle.top_of_block_orders.into_iter().for_each(|order| {
                if self.apply_filter_tob(&order, pool_stores) {
                    all_orders.push(HistoricalOrders::TOB(order))
                }
            });
        }

        if self.order_kinds.contains(&OrderKind::User)
            || self.order_kinds.contains(&OrderKind::None)
        {
            bundle.user_orders.into_iter().for_each(|order| {
                if self.apply_filter_user(&order, pool_stores) {
                    all_orders.push(HistoricalOrders::User(order))
                }
            });
        }

        all_orders
    }

    fn apply_filter_tob(
        &self,
        order: &TopOfBlockOrder,
        pool_stores: &AngstromPoolTokenIndexToPair
    ) -> bool {
        if self.order_filters.contains(&OrderFilter::None) {
            return true;
        }

        self.order_filters.iter().all(|filter| match filter {
            OrderFilter::ByPoolId { pool_id } => {
                if let Some(pool) = pool_stores.0.get(&order.pairs_index) {
                    pool.pool_id == *pool_id
                } else {
                    false
                }
            }
            OrderFilter::ByTokens { token0, token1 } => {
                if let Some(pool) = pool_stores.0.get(&order.pairs_index) {
                    pool.token0 == *token0 && pool.token1 == *token1
                } else {
                    false
                }
            }
            OrderFilter::None => unreachable!()
        })
    }

    fn apply_filter_user(
        &self,
        order: &UserOrder,
        pool_stores: &AngstromPoolTokenIndexToPair
    ) -> bool {
        if self.order_filters.contains(&OrderFilter::None) {
            return true;
        }

        self.order_filters.iter().all(|filter| match filter {
            OrderFilter::ByPoolId { pool_id } => {
                if let Some(pool) = pool_stores.0.get(&order.pair_index) {
                    pool.pool_id == *pool_id
                } else {
                    false
                }
            }
            OrderFilter::ByTokens { token0, token1 } => {
                if let Some(pool) = pool_stores.0.get(&order.pair_index) {
                    pool.token0 == *token0 && pool.token1 == *token1
                } else {
                    false
                }
            }
            OrderFilter::None => unreachable!()
        })
    }
}

#[derive(Debug, Copy, Hash, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub enum OrderKind {
    TOB,
    User,
    None
}

impl FromStr for OrderKind {
    type Err = eyre::ErrReport;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower_s = s.to_lowercase();
        match lower_s.as_str() {
            "tob" => Ok(Self::TOB),
            "user" => Ok(Self::User),
            "none" => Ok(Self::None),
            _ => Err(eyre::eyre!("{s} is not a valid OrderKind"))
        }
    }
}

#[derive(Debug, Copy, Hash, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub enum OrderFilter {
    ByPoolId { pool_id: PoolId },
    ByTokens { token0: Address, token1: Address },
    None
}

impl OrderFilter {
    fn addresses(&self) -> Option<(Address, Address)> {
        if let OrderFilter::ByTokens { token0, token1 } = self {
            Some((*token0, *token1))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum HistoricalOrders {
    TOB(TopOfBlockOrder),
    User(UserOrder)
}

#[derive(Debug)]
pub(crate) struct AngstromPoolTokenIndexToPair(HashMap<u16, PoolMetadata>);

impl AngstromPoolTokenIndexToPair {
    pub(crate) async fn new_with_tokens<P>(
        provider: &EthRpcProvider<P>,
        filter: &HistoricalOrdersFilter
    ) -> eyre::Result<Self>
    where
        P: Provider + Clone
    {
        let token_pairs = filter
            .order_filters
            .iter()
            .flat_map(|filter| filter.addresses());

        if token_pairs.clone().count() == 0 {
            return Ok(Self(HashMap::default()));
        }

        let config_store = pool_config_store(provider.provider()).await?;
        let pools = token_pairs
            .map(|(token0, token1)| {
                let pool_config = config_store.get_entry(token0, token1).ok_or(eyre::eyre!(
                    "no config store entry for tokens {token0:?} - {token1:?}"
                ))?;
                let pool_meta = PoolMetadata::new(token0, token1, pool_config);
                Ok::<_, eyre::ErrReport>((pool_meta.storage_idx as u16, pool_meta))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(Self(pools))
    }
}
