use std::ops::Deref;

use alloy_consensus::BlockHeader;
use alloy_eips::BlockId;
use alloy_network::{BlockResponse, Ethereum, Network, ReceiptResponse, TransactionBuilder};
use alloy_primitives::{Address, StorageKey, StorageValue, TxHash};
use alloy_provider::{DynProvider, Provider, RootProvider};
use alloy_rpc_types::{BlockTransactionsKind, Filter, Log};
use alloy_sol_types::{SolCall, SolType};
use uniswap_storage::StorageSlotFetcher;

use crate::types::providers::primitive_fetcher::PrimitivesFetcher;

/// Wrapper for alloy providers that implements SDK traits.
/// This wrapper is necessary to avoid trait coherence conflicts with
/// `RethDbProviderWrapper`.
#[derive(Debug, Clone)]
pub struct AlloyProviderWrapper<N: Network = Ethereum> {
    provider: DynProvider<N>
}

impl<N: Network> AlloyProviderWrapper<N> {
    pub fn new(provider: impl Provider<N> + 'static) -> Self {
        Self { provider: DynProvider::new(provider) }
    }

    pub fn provider(&self) -> &DynProvider<N> {
        &self.provider
    }

    pub fn into_inner(self) -> DynProvider<N> {
        self.provider
    }
}

impl<N: Network> Deref for AlloyProviderWrapper<N> {
    type Target = DynProvider<N>;

    fn deref(&self) -> &Self::Target {
        &self.provider
    }
}

impl<N: Network> Provider<N> for AlloyProviderWrapper<N> {
    fn root(&self) -> &RootProvider<N> {
        self.provider.root()
    }
}

#[async_trait::async_trait]
impl<N: Network> StorageSlotFetcher for AlloyProviderWrapper<N> {
    async fn storage_at(
        &self,
        address: Address,
        key: StorageKey,
        block_id: BlockId
    ) -> eyre::Result<StorageValue> {
        Ok(self
            .root()
            .get_storage_at(address, key.into())
            .block_id(block_id)
            .await?)
    }
}

#[async_trait::async_trait]
impl<N: Network> PrimitivesFetcher<N> for AlloyProviderWrapper<N>
where
    DynProvider<N>: Provider<N>
{
    async fn fetch_logs_primitive(&self, filter: &Filter) -> eyre::Result<Vec<Log>> {
        Ok(self.provider.get_logs(filter).await?)
    }

    async fn view_call<IC>(
        &self,
        block_id: BlockId,
        contract: Address,
        call: IC
    ) -> eyre::Result<IC::Return>
    where
        IC: SolCall + Send
    {
        let mut tx = N::TransactionRequest::default();
        tx.set_to(contract);
        tx.set_input(call.abi_encode());

        let data = self.call(tx).block(block_id).await?;
        Ok(IC::abi_decode_returns(&data)?)
    }

    async fn view_deploy_call<IC>(
        &self,
        block_id: BlockId,
        tx: <N as Network>::TransactionRequest
    ) -> eyre::Result<IC::RustType>
    where
        IC: SolType + Send
    {
        let data = self.call(tx).block(block_id).await?;
        Ok(IC::abi_decode(&data)?)
    }

    async fn alloy_root_provider(&self) -> eyre::Result<RootProvider<N>> {
        Ok(self.provider.root().clone())
    }

    async fn block_number_from_block_id(&self, block_id: BlockId) -> eyre::Result<u64> {
        let number = if let Some(b) = block_id.as_u64() {
            b
        } else {
            self.get_block(block_id)
                .await?
                .ok_or_else(|| eyre::eyre!("block not found: {block_id:?}"))?
                .header()
                .number()
        };

        Ok(number)
    }

    async fn fetch_block_primitive(
        &self,
        block_id: BlockId,
        full: bool
    ) -> eyre::Result<<N as Network>::BlockResponse> {
        let tx_kind =
            if full { BlockTransactionsKind::Full } else { BlockTransactionsKind::Hashes };
        self.get_block(block_id)
            .kind(tx_kind)
            .await?
            .ok_or_else(|| eyre::eyre!("block does not exist: {block_id:?}"))
    }

    async fn tx_success(&self, tx_hash: TxHash) -> eyre::Result<bool> {
        Ok(self
            .provider
            .get_transaction_receipt(tx_hash)
            .await?
            .ok_or_else(|| eyre::eyre!("tx does not exist: {tx_hash:?}"))?
            .status())
    }

    async fn tx_by_hash_primitive(
        &self,
        tx_hash: TxHash
    ) -> eyre::Result<Option<<N as Network>::TransactionResponse>> {
        Ok(self.provider.get_transaction_by_hash(tx_hash).await?)
    }
}
