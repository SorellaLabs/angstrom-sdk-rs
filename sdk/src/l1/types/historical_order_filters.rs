use std::{
    collections::{HashMap, HashSet},
    str::FromStr
};

use alloy_primitives::Address;
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager::PoolKey,
    contract_payloads::angstrom::{AngstromBundle, TopOfBlockOrder, UserOrder},
    primitive::PoolId
};

use crate::{l1::apis::data_api::AngstromL1DataApi, types::common::*};

#[derive(Debug, Default, Clone)]
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

    pub(crate) fn filter_bundle(
        &self,
        bundle: AngstromBundle,
        pool_stores: &AngstromPoolTokenIndexToPair
    ) -> Vec<HistoricalOrders> {
        let mut all_orders = Vec::new();

        if self.order_kinds.contains(&OrderKind::TOB) || self.order_kinds.is_empty() {
            bundle.top_of_block_orders.into_iter().for_each(|order| {
                if self.apply(order.pairs_index, pool_stores) {
                    all_orders.push(HistoricalOrders::TOB(order))
                }
            });
        }

        if self.order_kinds.contains(&OrderKind::User) || self.order_kinds.is_empty() {
            bundle.user_orders.into_iter().for_each(|order| {
                if self.apply(order.pair_index, pool_stores) {
                    all_orders.push(HistoricalOrders::User(order))
                }
            });
        }

        all_orders
    }

    fn apply(&self, order_pair_index: u16, pool_stores: &AngstromPoolTokenIndexToPair) -> bool {
        if self.order_filters.is_empty() {
            return true;
        }

        self.order_filters.iter().all(|filter| match filter {
            OrderFilter::PoolId(pool_id) => pool_stores
                .0
                .get(&order_pair_index)
                .map(|pool| *pool.pool_id == *pool_id)
                .unwrap_or_default(),
            OrderFilter::PoolKey(pool_key) => pool_stores
                .0
                .get(&order_pair_index)
                .map(|pool| pool.pool_key == *pool_key)
                .unwrap_or_default(),
            OrderFilter::Tokens(token0, token1) => pool_stores
                .0
                .get(&order_pair_index)
                .map(|pool| pool.token0 == *token0 && pool.token1 == *token1)
                .unwrap_or_default()
        })
    }
}

#[derive(Debug, Copy, Hash, Clone, PartialEq, Eq)]
pub enum OrderKind {
    TOB,
    User
}

impl FromStr for OrderKind {
    type Err = eyre::ErrReport;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower_s = s.to_lowercase();
        match lower_s.as_str() {
            "tob" => Ok(Self::TOB),
            "user" => Ok(Self::User),
            _ => Err(eyre::eyre!("{s} is not a valid OrderKind"))
        }
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum OrderFilter {
    PoolId(PoolId),
    PoolKey(PoolKey),
    Tokens(Address, Address)
}

impl OrderFilter {
    fn addresses(&self) -> Option<(Address, Address)> {
        if let OrderFilter::Tokens(token0, token1) = self { Some((*token0, *token1)) } else { None }
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
        provider: &P,
        filter: &HistoricalOrdersFilter
    ) -> eyre::Result<Self>
    where
        P: AngstromL1DataApi + Sized
    {
        let token_pairs = filter
            .order_filters
            .iter()
            .flat_map(|filter| filter.addresses());

        if token_pairs.clone().count() == 0 {
            return Ok(Self(HashMap::default()));
        }

        let config_store = provider.pool_config_store(filter.from_block).await?;
        let pools = token_pairs
            .map(|(token0, token1)| {
                config_store
                    .get_entry(token0, token1)
                    .ok_or(eyre::eyre!("no config store entry for tokens {token0:?} - {token1:?}"))
                    .map(|cng| (cng.store_index as u16, PoolMetadata::new(token0, token1, cng)))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(Self(pools))
    }
}
