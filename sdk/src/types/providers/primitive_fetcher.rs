use std::fmt::Debug;

use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{Address, TxHash};
use alloy_provider::RootProvider;
use alloy_rpc_types::{Filter, Log};
use alloy_sol_types::{SolCall, SolType};

#[async_trait::async_trait]
#[auto_impl::auto_impl(&, Box, Arc)]
pub trait PrimitivesFetcher<N: Network>: Send + Sync {
    async fn fetch_logs_primitive(&self, filter: &Filter) -> eyre::Result<Vec<Log>>;

    async fn view_call<IC>(
        &self,
        block_id: BlockId,
        contract: Address,
        call: IC
    ) -> eyre::Result<IC::Return>
    where
        IC: SolCall + Send + Debug;

    async fn view_deploy_call<IC>(
        &self,
        block_id: BlockId,
        tx: <N as Network>::TransactionRequest
    ) -> eyre::Result<IC::RustType>
    where
        IC: SolType + Send;

    async fn alloy_root_provider(&self) -> eyre::Result<RootProvider<N>>;

    async fn block_number_from_block_id(&self, block_id: BlockId) -> eyre::Result<u64>;

    async fn fetch_block_primitive(
        &self,
        block_id: BlockId,
        full: bool
    ) -> eyre::Result<<N as Network>::BlockResponse>;

    async fn tx_success_primitive(&self, tx_hash: TxHash) -> eyre::Result<bool>;

    async fn tx_by_hash_primitive(
        &self,
        tx_hash: TxHash
    ) -> eyre::Result<Option<<N as Network>::TransactionResponse>>;
}
