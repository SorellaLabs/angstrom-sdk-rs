use std::sync::Arc;

use alloy_eips::BlockId;
use alloy_network::{Network, ReceiptResponse, TransactionBuilder};
use alloy_primitives::{Address, Bytes, TxHash, TxKind};
use alloy_provider::RootProvider;
use alloy_rpc_types::{Block, Filter, Log, TransactionRequest};
use alloy_sol_types::{SolCall, SolType};
use eth_network_exts::EthNetworkExt;
use lib_reth::{
    EthApiTypes, ExecuteEvm,
    helpers::{EthBlocks, EthTransactions},
    reth_libmdbx::{NodeClientSpec, RethNodeClient},
    traits::{
        EthRevm, EthRevmParams, EthStream, OpTransaction, empty_mainnet_revm, empty_op_mainnet_revm
    }
};
use revm::context::TxEnv;

use crate::types::providers::{AlloyProviderWrapper, primitive_fetcher::PrimitivesFetcher};

#[derive(Clone)]
pub struct RethDbProviderWrapper<N>
where
    N: EthNetworkExt,
    N::RethNode: NodeClientSpec
{
    provider: Arc<RethNodeClient<N>>
}

impl<N> RethDbProviderWrapper<N>
where
    N: EthNetworkExt,
    N::RethNode: NodeClientSpec
{
    pub fn new(provider: Arc<RethNodeClient<N>>) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> Arc<RethNodeClient<N>> {
        self.provider.clone()
    }

    pub fn provider_ref(&self) -> &RethNodeClient<N> {
        &self.provider
    }
}

#[async_trait::async_trait]
impl<N> PrimitivesFetcher<N::AlloyNetwork> for RethDbProviderWrapper<N>
where
    N: EthNetworkExt,
    N::RethNode: NodeClientSpec,
    <N::RethNode as NodeClientSpec>::Api: EthApiTypes<NetworkTypes = N::AlloyNetwork>,
    <N::AlloyNetwork as Network>::TransactionRequest:
        AsRef<TransactionRequest> + AsMut<TransactionRequest>,
    N::AlloyNetwork: Network<
        BlockResponse = Block<
            <N::AlloyNetwork as Network>::TransactionResponse,
            <N::AlloyNetwork as Network>::HeaderResponse
        >
    >
{
    async fn fetch_logs_primitive(&self, filter: &Filter) -> eyre::Result<Vec<Log>> {
        Ok(AlloyProviderWrapper::new(self.alloy_root_provider().await?)
            .fetch_logs_primitive(filter)
            .await?)
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
        let chain_id = <N as EthNetworkExt>::CHAIN_ID;

        let tx = TxEnv {
            kind: TxKind::Call(contract),
            data: call.abi_encode().into(),
            chain_id: Some(chain_id),
            ..Default::default()
        };

        let evm_db = self
            .provider
            .make_cache_db(&EthRevmParams { block_id, chain_id })?;

        let data = if N::is_op_chain() {
            let mut evm = empty_op_mainnet_revm(evm_db, chain_id, true);

            let mut tx = OpTransaction::new(tx);
            tx.enveloped_tx = Some(Bytes::from_iter([0x00]));
            evm.transact(tx)?.result.into_output()
        } else {
            let mut evm = empty_mainnet_revm(evm_db, chain_id, true);
            evm.transact(tx)?.result.into_output()
        };

        Ok(IC::abi_decode_returns(&data.unwrap_or_default())?)
    }

    async fn view_deploy_call<IC>(
        &self,
        block_id: BlockId,
        tx: <N::AlloyNetwork as Network>::TransactionRequest
    ) -> eyre::Result<IC::RustType>
    where
        IC: SolType + Send
    {
        let chain_id = <N as EthNetworkExt>::CHAIN_ID;

        let tx = TxEnv {
            kind: TxKind::Create,
            data: tx.input().cloned().unwrap_or_default(),
            chain_id: Some(chain_id),
            ..Default::default()
        };

        let evm_db = self
            .provider
            .make_cache_db(&EthRevmParams { block_id, chain_id })?;

        let data = if N::is_op_chain() {
            let mut evm = empty_op_mainnet_revm(evm_db, chain_id, true);

            let mut tx = OpTransaction::new(tx);
            tx.enveloped_tx = Some(Bytes::from_iter([0x00]));
            evm.transact(tx)?.result.into_output()
        } else {
            let mut evm = empty_mainnet_revm(evm_db, chain_id, true);
            evm.transact(tx)?.result.into_output()
        };

        Ok(IC::abi_decode(&data.unwrap_or_default())?)
    }

    async fn alloy_root_provider(&self) -> eyre::Result<RootProvider<N::AlloyNetwork>> {
        Ok(self.provider().root_provider().await?)
    }

    async fn block_number_from_block_id(&self, block_id: BlockId) -> eyre::Result<u64> {
        let number = if let Some(b) = block_id.as_u64() {
            b
        } else {
            EthBlocks::rpc_block(&self.provider.eth_api(), block_id, false)
                .await?
                .ok_or_else(|| eyre::eyre!("block not found: {block_id:?}"))?
                .number()
        };

        Ok(number)
    }

    async fn fetch_block_primitive(
        &self,
        block_id: BlockId,
        full: bool
    ) -> eyre::Result<<N::AlloyNetwork as Network>::BlockResponse> {
        EthBlocks::rpc_block(&self.provider.eth_api(), block_id, full)
            .await?
            .ok_or_else(|| eyre::eyre!("block does not exist: {block_id:?}"))
    }

    async fn tx_success_primitive(&self, tx_hash: TxHash) -> eyre::Result<bool> {
        Ok(EthTransactions::transaction_receipt(&self.provider.eth_api(), tx_hash)
            .await?
            .ok_or_else(|| eyre::eyre!("tx does not exist: {tx_hash:?}"))?
            .status())
    }

    async fn tx_by_hash_primitive(
        &self,
        tx_hash: TxHash
    ) -> eyre::Result<Option<<N::AlloyNetwork as Network>::TransactionResponse>> {
        let api = self.provider.eth_api();

        Ok(EthTransactions::transaction_by_hash(&api, tx_hash)
            .await?
            .map(|tx| tx.into_transaction(api.converter()))
            .transpose()
            .map_err(<<N::RethNode as NodeClientSpec>::Api as EthApiTypes>::Error::from)?)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;
    use angstrom_types_primitives::ERC20;
    use revm_database::{CacheDB, EmptyDB};

    use super::*;

    #[test]
    fn test_make_call() {
        let db = EmptyDB::new();

        let chain_id = 8453;

        let mut tx_env = TxEnv {
            kind: TxKind::Call(address!("0x4200000000000000000000000000000000000006")),
            data: ERC20::decimalsCall {}.abi_encode().into(),
            // chain_id: Some(chain_id),
            ..Default::default()
        };

        let mut evm = empty_op_mainnet_revm(CacheDB::new(db), chain_id, true);

        let mut tx = OpTransaction::new(tx_env.clone());
        tx.enveloped_tx = Some(Bytes::default());
        let res = evm.transact(tx);
        assert!(res.is_err());

        tx_env.chain_id = Some(chain_id);
        let mut tx = OpTransaction::new(tx_env);
        tx.enveloped_tx = Some(Bytes::default());
        let res = evm.transact(tx);
        assert!(res.is_ok());
    }
}
