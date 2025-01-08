use std::collections::HashMap;

use alloy_consensus::BlobTransactionSidecar;
use alloy_eips::eip4844::BYTES_PER_BLOB;
use alloy_primitives::{
    aliases::{I24, U24},
    ruint::aliases::B384,
    Address, Bytes, ChainId, FixedBytes, TxKind, B256, B512, U256
};
use alloy_rpc_types::{AccessList, AccessListItem, TransactionInput, TransactionRequest};
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

use crate::{
    apis::order_builder::add_liquidity,
    types::{HistoricalOrders, TransactionRequestWithLiquidityMeta}
};

#[derive(Debug, Clone, NeonObject)]
pub struct OrderBuilderAddLiquidityArgs {
    token0:                   Address,
    token1:                   Address,
    tick_lower:               i32,
    tick_upper:               i32,
    liquidity:                U256,
    max_fee_per_gas:          Option<u128>,
    max_priority_fee_per_gas: Option<u128>
}

impl OrderBuilderAddLiquidityArgs {
    pub fn add_liquidity(self) -> TransactionRequestWithLiquidityMeta {
        add_liquidity(
            self.token0,
            self.token1,
            self.tick_lower,
            self.tick_upper,
            self.liquidity,
            self.max_fee_per_gas,
            self.max_priority_fee_per_gas
        )
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct TransactionRequestWithLiquidityMetaNeon {
    tx_request: TransactionRequestNeon,
    tokens:     (Address, Address),
    tick_lower: i32,
    tick_upper: i32,
    liquidity:  U256,
    is_add:     bool
}

#[derive(Debug, Clone, NeonObject)]
struct TransactionRequestNeon {
    from: Option<Address>,
    to: Option<TxKind>,
    gas_price: Option<u128>,
    max_fee_per_gas: Option<u128>,
    max_priority_fee_per_gas: Option<u128>,
    max_fee_per_blob_gas: Option<u128>,
    gas: Option<u64>,
    value: Option<U256>,
    input: TransactionInputNeon,
    nonce: Option<u64>,
    chain_id: Option<ChainId>,
    access_list: Option<AccessList>,
    transaction_type: Option<u8>,
    blob_versioned_hashes: Option<Vec<B256>>,
    sidecar: Option<BlobTransactionSidecar>,
    authorization_list: Option<Vec<SignedAuthorization>>
}

#[derive(Debug, Clone, NeonObject)]
enum TxKindNeon {
    Create,
    Call { address: Address }
}

#[derive(Debug, Clone, NeonObject)]
struct TransactionInputNeon {
    input: Option<Bytes>,
    data:  Option<Bytes>
}

#[derive(Debug, Clone, NeonObject)]
struct AccessListNeon {
    list: Vec<AccessListItemNeon>
}

#[derive(Debug, Clone, NeonObject)]
struct AccessListItemNeon {
    address:      Address,
    storage_keys: Vec<B256>
}

#[derive(Debug, Clone, NeonObject)]
pub struct BlobTransactionSidecarNeon {
    pub blobs:       Vec<FixedBytes<BYTES_PER_BLOB>>,
    pub commitments: Vec<FixedBytes<48>>,
    pub proofs:      Vec<FixedBytes<48>>
}
