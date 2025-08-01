use alloy_primitives::{
    Address, TxHash,
    aliases::{I24, U24},
    keccak256
};
use alloy_sol_types::SolValue;
use angstrom_types::{
    contract_bindings::pool_manager::PoolManager::PoolKey,
    contract_payloads::angstrom::AngPoolConfigEntry,
    primitive::{ANGSTROM_ADDRESS, PoolId}
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenPair {
    pub token0: Address,
    pub token1: Address
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfoWithMeta {
    pub address: Address,
    pub symbol:  String
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct PoolMetadata {
    pub pool_key:     PoolKey,
    pub pool_id:      PoolId,
    pub token0:       Address,
    pub token1:       Address,
    pub fee:          u32,
    pub tick_spacing: u16,
    pub storage_idx:  u64
}

impl PoolMetadata {
    pub fn new(token0: Address, token1: Address, config_store: AngPoolConfigEntry) -> Self {
        let pool_key = PoolKey {
            currency0:   token0,
            currency1:   token1,
            fee:         U24::from(config_store.fee_in_e6),
            tickSpacing: I24::unchecked_from(config_store.tick_spacing),
            hooks:       *ANGSTROM_ADDRESS.get().unwrap()
        };

        Self {
            token0,
            token1,
            pool_key,
            pool_id: pool_key.into(),
            fee: config_store.fee_in_e6,
            tick_spacing: config_store.tick_spacing,
            storage_idx: config_store.store_index as u64
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TokensOrPoolId {
    Tokens(Address, Address),
    PoolId(PoolId)
}

impl From<PoolId> for TokensOrPoolId {
    fn from(value: PoolId) -> Self {
        TokensOrPoolId::PoolId(value)
    }
}

impl From<(Address, Address)> for TokensOrPoolId {
    fn from(value: (Address, Address)) -> Self {
        let (t0, t1) = if value.0 > value.1 { (value.1, value.0) } else { (value.0, value.1) };

        TokensOrPoolId::Tokens(t0, t1)
    }
}

pub(crate) fn sort_tokens(token0: Address, token1: Address) -> (Address, Address) {
    if token0 < token1 { (token0, token1) } else { (token1, token0) }
}

#[derive(Debug, Clone)]
pub struct BinanceTokenPrice {
    pub address:   Address,
    pub price:     Option<f64>,
    pub error_msg: Option<String>
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash)]
pub struct PoolKeyWithAngstromFee {
    pub pool_key:       PoolKey,
    pub pool_fee_in_e6: U24
}

impl PoolKeyWithAngstromFee {
    pub fn as_angstrom_pool_id(&self) -> PoolId {
        let mut this = *self;
        this.pool_key.fee = this.pool_fee_in_e6;
        this.pool_key.into()
    }

    pub(crate) fn as_angstrom_pool_key_type(
        &self
    ) -> angstrom_types::contract_bindings::angstrom::Angstrom::PoolKey {
        angstrom_types::contract_bindings::angstrom::Angstrom::PoolKey {
            currency0:   self.pool_key.currency0,
            currency1:   self.pool_key.currency1,
            fee:         self.pool_fee_in_e6,
            tickSpacing: self.pool_key.tickSpacing,
            hooks:       self.pool_key.hooks
        }
    }
}

impl From<PoolKeyWithAngstromFee> for PoolId {
    fn from(value: PoolKeyWithAngstromFee) -> Self {
        keccak256(value.pool_key.abi_encode())
    }
}

impl From<&PoolKeyWithAngstromFee> for PoolId {
    fn from(value: &PoolKeyWithAngstromFee) -> Self {
        keccak256(value.pool_key.abi_encode())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WithEthMeta<D> {
    pub block_number: Option<u64>,
    pub tx_hash:      Option<TxHash>,
    pub tx_idx:       Option<u64>,
    pub inner:        D
}

impl<D> WithEthMeta<D> {
    pub fn new(
        block_number: Option<u64>,
        tx_hash: Option<TxHash>,
        tx_idx: Option<u64>,
        inner: D
    ) -> Self {
        Self { block_number, tx_hash, inner, tx_idx }
    }

    pub fn map_inner<O>(self, f: impl Fn(D) -> O) -> WithEthMeta<O> {
        WithEthMeta {
            block_number: self.block_number,
            tx_hash:      self.tx_hash,
            tx_idx:       self.tx_idx,
            inner:        f(self.inner)
        }
    }
}
