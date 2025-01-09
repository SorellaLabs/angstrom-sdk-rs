use alloy_consensus::BlobTransactionSidecar;
use alloy_eips::{eip4844::BYTES_PER_BLOB, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, ChainId, FixedBytes, TxKind, B256, U256};
use alloy_rpc_types::{
    AccessList, AccessListItem, Authorization, TransactionInput, TransactionRequest
};
use angstrom_sdk_macros::{neon_object_as, NeonObject};
use angstrom_types::sol_bindings::rpc_orders::{
    ExactFlashOrder, ExactStandingOrder, PartialFlashOrder, PartialStandingOrder, TopOfBlockOrder
};
use neon::object::Object;

use crate::{
    apis::order_builder::{
        add_liquidity, exact_flash_order, exact_standing_order, partial_flash_order,
        partial_standing_order, remove_liquidity, top_of_block_order
    },
    types::TransactionRequestWithLiquidityMeta
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
pub struct OrderBuilderRemoveLiquidityArgs {
    token0:                   Address,
    token1:                   Address,
    tick_lower:               i32,
    tick_upper:               i32,
    liquidity:                U256,
    max_fee_per_gas:          Option<u128>,
    max_priority_fee_per_gas: Option<u128>
}

impl OrderBuilderRemoveLiquidityArgs {
    pub fn remove_liquidity(self) -> TransactionRequestWithLiquidityMeta {
        remove_liquidity(
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
pub struct OrderBuilderTopOfBlockOrderArgs {
    asset_in:        Address,
    asset_out:       Address,
    quantity_in:     u128,
    quantity_out:    u128,
    max_gas_asset0:  u128,
    valid_for_block: u64
}

impl OrderBuilderTopOfBlockOrderArgs {
    pub fn top_of_block_order(self) -> TopOfBlockOrder {
        top_of_block_order(
            self.asset_in,
            self.asset_out,
            self.quantity_in,
            self.quantity_out,
            self.max_gas_asset0,
            self.valid_for_block
        )
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct OrderBuilderPartialStandingOrderArgs {
    asset_in:             Address,
    asset_out:            Address,
    min_amount_in:        u128,
    max_amount_in:        u128,
    min_price:            U256,
    max_extra_fee_asset0: Option<u128>,
    deadline:             Option<u64>
}

impl OrderBuilderPartialStandingOrderArgs {
    pub fn partial_standing_order(self) -> PartialStandingOrder {
        partial_standing_order(
            self.asset_in,
            self.asset_out,
            self.min_amount_in,
            self.max_amount_in,
            self.min_price,
            self.max_extra_fee_asset0,
            self.deadline
        )
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct OrderBuilderExactStandingOrderArgs {
    asset_in:             Address,
    asset_out:            Address,
    exact_in:             bool,
    amount:               u128,
    min_price:            U256,
    max_extra_fee_asset0: Option<u128>,
    deadline:             Option<u64>
}

impl OrderBuilderExactStandingOrderArgs {
    pub fn exact_standing_order(self) -> ExactStandingOrder {
        exact_standing_order(
            self.asset_in,
            self.asset_out,
            self.exact_in,
            self.amount,
            self.min_price,
            self.max_extra_fee_asset0,
            self.deadline
        )
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct OrderBuilderPartialFlashOrderArgs {
    asset_in:             Address,
    asset_out:            Address,
    min_amount_in:        u128,
    max_amount_in:        u128,
    min_price:            U256,
    max_extra_fee_asset0: Option<u128>,
    valid_for_block:      u64
}

impl OrderBuilderPartialFlashOrderArgs {
    pub fn partial_flash_order(self) -> PartialFlashOrder {
        partial_flash_order(
            self.asset_in,
            self.asset_out,
            self.min_amount_in,
            self.max_amount_in,
            self.min_price,
            self.max_extra_fee_asset0,
            self.valid_for_block
        )
    }
}

#[derive(Debug, Clone, NeonObject)]
pub struct OrderBuilderExactFlashOrderArgs {
    asset_in:             Address,
    asset_out:            Address,
    exact_in:             bool,
    amount:               u128,
    min_price:            U256,
    max_extra_fee_asset0: Option<u128>,
    valid_for_block:      u64
}

impl OrderBuilderExactFlashOrderArgs {
    pub fn exact_flash_order(self) -> ExactFlashOrder {
        exact_flash_order(
            self.asset_in,
            self.asset_out,
            self.exact_in,
            self.amount,
            self.min_price,
            self.max_extra_fee_asset0,
            self.valid_for_block
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

impl From<TransactionRequestWithLiquidityMeta> for TransactionRequestWithLiquidityMetaNeon {
    fn from(value: TransactionRequestWithLiquidityMeta) -> Self {
        Self {
            tx_request: value.tx_request.into(),
            tokens:     value.tokens,
            tick_lower: value.tick_lower,
            tick_upper: value.tick_upper,
            liquidity:  value.liquidity,
            is_add:     value.is_add
        }
    }
}

impl Into<TransactionRequestWithLiquidityMeta> for TransactionRequestWithLiquidityMetaNeon {
    fn into(self) -> TransactionRequestWithLiquidityMeta {
        TransactionRequestWithLiquidityMeta {
            tx_request: self.tx_request.into(),
            tokens:     self.tokens,
            tick_lower: self.tick_lower,
            tick_upper: self.tick_upper,
            liquidity:  self.liquidity,
            is_add:     self.is_add
        }
    }
}

neon_object_as!(TransactionRequestWithLiquidityMeta, TransactionRequestWithLiquidityMetaNeon);

#[derive(Debug, Clone, NeonObject)]
struct TransactionRequestNeon {
    from: Option<Address>,
    to: Option<TxKindNeon>,
    gas_price: Option<u128>,
    max_fee_per_gas: Option<u128>,
    max_priority_fee_per_gas: Option<u128>,
    max_fee_per_blob_gas: Option<u128>,
    gas: Option<u64>,
    value_: Option<U256>,
    input: TransactionInputNeon,
    nonce: Option<u64>,
    chain_id: Option<ChainId>,
    access_list: Option<AccessListNeon>,
    transaction_type: Option<u8>,
    blob_versioned_hashes: Option<Vec<B256>>,
    sidecar: Option<BlobTransactionSidecarNeon>,
    authorization_list: Option<Vec<SignedAuthorizationNeon>>
}

impl From<TransactionRequest> for TransactionRequestNeon {
    fn from(value: TransactionRequest) -> Self {
        Self {
            from: value.from,
            to: value.to.map(Into::into),
            gas_price: value.gas_price,
            max_fee_per_gas: value.max_fee_per_gas,
            max_priority_fee_per_gas: value.max_priority_fee_per_gas,
            max_fee_per_blob_gas: value.max_fee_per_blob_gas,
            gas: value.gas,
            value_: value.value,
            input: value.input.into(),
            nonce: value.nonce,
            chain_id: value.chain_id,
            access_list: value.access_list.map(Into::into),
            transaction_type: value.transaction_type,
            blob_versioned_hashes: value.blob_versioned_hashes,
            sidecar: value.sidecar.map(Into::into),
            authorization_list: value
                .authorization_list
                .map(|list| list.into_iter().map(Into::into).collect())
        }
    }
}

impl Into<TransactionRequest> for TransactionRequestNeon {
    fn into(self) -> TransactionRequest {
        TransactionRequest {
            from: self.from,
            to: self.to.map(Into::into),
            gas_price: self.gas_price,
            max_fee_per_gas: self.max_fee_per_gas,
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            max_fee_per_blob_gas: self.max_fee_per_blob_gas,
            gas: self.gas,
            value: self.value_,
            input: self.input.into(),
            nonce: self.nonce,
            chain_id: self.chain_id,
            access_list: self.access_list.map(Into::into),
            transaction_type: self.transaction_type,
            blob_versioned_hashes: self.blob_versioned_hashes,
            sidecar: self.sidecar.map(Into::into),
            authorization_list: self
                .authorization_list
                .map(|list| list.into_iter().map(Into::into).collect())
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
enum TxKindNeon {
    Create,
    Call { address: Address }
}

// TxKind conversions
impl From<TxKind> for TxKindNeon {
    fn from(value: TxKind) -> Self {
        match value {
            TxKind::Create => Self::Create,
            TxKind::Call(address) => Self::Call { address }
        }
    }
}

impl Into<TxKind> for TxKindNeon {
    fn into(self) -> TxKind {
        match self {
            Self::Create => TxKind::Create,
            Self::Call { address } => TxKind::Call(address)
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
struct TransactionInputNeon {
    input: Option<Bytes>,
    data:  Option<Bytes>
}

impl From<TransactionInput> for TransactionInputNeon {
    fn from(value: TransactionInput) -> Self {
        Self { input: value.input, data: value.data }
    }
}

impl Into<TransactionInput> for TransactionInputNeon {
    fn into(self) -> TransactionInput {
        TransactionInput { input: self.input, data: self.data }
    }
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

impl From<AccessList> for AccessListNeon {
    fn from(value: AccessList) -> Self {
        Self { list: value.0.into_iter().map(Into::into).collect() }
    }
}

impl Into<AccessList> for AccessListNeon {
    fn into(self) -> AccessList {
        AccessList(self.list.into_iter().map(Into::into).collect())
    }
}

impl From<AccessListItem> for AccessListItemNeon {
    fn from(value: AccessListItem) -> Self {
        Self { address: value.address, storage_keys: value.storage_keys }
    }
}

impl Into<AccessListItem> for AccessListItemNeon {
    fn into(self) -> AccessListItem {
        AccessListItem { address: self.address, storage_keys: self.storage_keys }
    }
}

#[derive(Debug, Clone, NeonObject)]
struct BlobTransactionSidecarNeon {
    blobs:       Vec<FixedBytes<BYTES_PER_BLOB>>,
    commitments: Vec<FixedBytes<48>>,
    proofs:      Vec<FixedBytes<48>>
}

impl From<BlobTransactionSidecar> for BlobTransactionSidecarNeon {
    fn from(value: BlobTransactionSidecar) -> Self {
        Self { blobs: value.blobs, commitments: value.commitments, proofs: value.proofs }
    }
}

impl Into<BlobTransactionSidecar> for BlobTransactionSidecarNeon {
    fn into(self) -> BlobTransactionSidecar {
        BlobTransactionSidecar {
            blobs:       self.blobs,
            commitments: self.commitments,
            proofs:      self.proofs
        }
    }
}

#[derive(Debug, Clone, NeonObject)]
struct SignedAuthorizationNeon {
    inner:    AuthorizationNeon,
    y_parity: u8,
    r:        U256,
    s:        U256
}

impl From<SignedAuthorization> for SignedAuthorizationNeon {
    fn from(value: SignedAuthorization) -> Self {
        Self {
            inner:    value.inner().clone().into(),
            y_parity: value.y_parity(),
            r:        value.r(),
            s:        value.s()
        }
    }
}

impl Into<SignedAuthorization> for SignedAuthorizationNeon {
    fn into(self) -> SignedAuthorization {
        SignedAuthorization::new_unchecked(self.inner.into(), self.y_parity, self.r, self.s)
    }
}

#[derive(Debug, Clone, NeonObject)]
struct AuthorizationNeon {
    chain_id: u64,
    address:  Address,
    nonce:    u64
}

impl From<Authorization> for AuthorizationNeon {
    fn from(value: Authorization) -> Self {
        Self { chain_id: value.chain_id, address: value.address, nonce: value.nonce }
    }
}

impl Into<Authorization> for AuthorizationNeon {
    fn into(self) -> Authorization {
        Authorization { chain_id: self.chain_id, address: self.address, nonce: self.nonce }
    }
}
