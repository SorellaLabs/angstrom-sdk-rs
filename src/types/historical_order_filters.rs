use std::collections::HashSet;

use alloy_rpc_types::Block;
use angstrom_rpc::types::OrderSubscriptionFilter;
use angstrom_types::contract_payloads::angstrom::{AngstromBundle, TopOfBlockOrder, UserOrder};
use pade::PadeDecode;

#[derive(Debug, Default, Clone)]
pub struct HistoricalOrdersFilter {
    pub order_kinds: HashSet<OrderKindFilter>,
    pub order_filters: HashSet<OrderSubscriptionFilter>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
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

    pub fn order_filter(mut self, filter: OrderSubscriptionFilter) -> Self {
        self.order_filters.insert(filter);
        self
    }

    pub fn order_filters(
        mut self,
        filters: impl IntoIterator<Item = OrderSubscriptionFilter>,
    ) -> Self {
        self.order_filters.extend(filters);
        self
    }

    pub fn order_kind(mut self, order_kind: OrderKindFilter) -> Self {
        self.order_kinds.insert(order_kind);
        self
    }

    pub fn order_kinds(mut self, order_kinds: impl IntoIterator<Item = OrderKindFilter>) -> Self {
        self.order_kinds.extend(order_kinds);
        self
    }

    pub fn filter_block(&self, block: Block) -> Vec<HistoricalOrders> {
        block
            .transactions
            .into_transactions()
            .filter_map(|transaction| {
                let mut input: &[u8] = &transaction.input;
                AngstromBundle::pade_decode(&mut input, None).ok()
            })
            .flat_map(|bundle| self.apply_kinds(bundle))
            .collect()
    }

    fn apply_kinds(&self, bundle: AngstromBundle) -> Vec<HistoricalOrders> {
        let mut all_orders = Vec::new();

        if self.order_kinds.contains(&OrderKindFilter::TOB)
            || self.order_kinds.contains(&OrderKindFilter::None)
        {
            bundle.top_of_block_orders.into_iter().for_each(|order| {
                if self.apply_filter_tob(&order) {
                    all_orders.push(HistoricalOrders::TOB(order))
                }
            });
        }

        if self.order_kinds.contains(&OrderKindFilter::User)
            || self.order_kinds.contains(&OrderKindFilter::None)
        {
            bundle.user_orders.into_iter().for_each(|order| {
                if self.apply_filter_user(&order) {
                    all_orders.push(HistoricalOrders::User(order))
                }
            });
        }

        all_orders
    }

    fn apply_filter_tob(&self, order: &TopOfBlockOrder) -> bool {
        todo!()
    }

    fn apply_filter_user(&self, order: &UserOrder) -> bool {
        todo!()
    }
}

#[derive(Debug, Copy, Hash, Clone, PartialEq, Eq)]
pub enum OrderKindFilter {
    TOB,
    User,
    None,
}

#[derive(Debug)]
pub enum HistoricalOrders {
    TOB(TopOfBlockOrder),
    User(UserOrder),
}
