use std::sync::Arc;

use alloy_eips::BlockId;
use alloy_network::{Network, ReceiptResponse, TransactionBuilder};
use alloy_primitives::{Address, TxHash, TxKind};
use alloy_provider::RootProvider;
use alloy_rpc_types::{Block, Filter, Log, TransactionRequest};
use alloy_sol_types::{SolCall, SolType};
use eth_network_exts::EthNetworkExt;
use lib_reth::{
    EthApiTypes, ExecuteEvm,
    helpers::{EthBlocks, EthTransactions},
    reth_libmdbx::{NodeClientSpec, RethNodeClient},
    traits::{EthRevm, EthRevmParams, EthStream, RevmNetworkSpec}
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
    N::AlloyNetwork: RevmNetworkSpec,
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
        let tx = TxEnv {
            kind: TxKind::Call(contract),
            data: call.abi_encode().into(),
            ..Default::default()
        };

        #[cfg(feature = "l2")]
        let tx = <N::AlloyNetwork as RevmNetworkSpec>::convert_build_tx(tx, |_| {});
        #[cfg(not(feature = "l2"))]
        let tx = <N::AlloyNetwork as RevmNetworkSpec>::convert_build_tx(tx);

        let mut evm: <N::AlloyNetwork as RevmNetworkSpec>::EVM<_, _> =
            self.provider.make_empty_evm(&EthRevmParams {
                block_id,
                chain_id: <N as EthNetworkExt>::CHAIN_ID
            })?;

        let data = evm.transact(tx)?;

        Ok(IC::abi_decode_returns(data.result.output().unwrap_or_default())?)
    }

    async fn view_deploy_call<IC>(
        &self,
        block_id: BlockId,
        tx: <N::AlloyNetwork as Network>::TransactionRequest
    ) -> eyre::Result<IC::RustType>
    where
        IC: SolType + Send
    {
        let call_data = tx.input().cloned().unwrap_or_default();
        let tx = TxEnv { kind: TxKind::Create, data: call_data, ..Default::default() };

        #[cfg(feature = "l2")]
        let tx = <N::AlloyNetwork as RevmNetworkSpec>::convert_build_tx(tx, |_| {});
        #[cfg(not(feature = "l2"))]
        let tx = <N::AlloyNetwork as RevmNetworkSpec>::convert_build_tx(tx);

        let mut evm: N::EVM<_, _> = self.provider.make_empty_evm(&EthRevmParams {
            block_id,
            chain_id: <N as EthNetworkExt>::CHAIN_ID
        })?;

        let data = evm.transact(tx)?;

        Ok(IC::abi_decode(data.result.output().unwrap_or_default())?)
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
