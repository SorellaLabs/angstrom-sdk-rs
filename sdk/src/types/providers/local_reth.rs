use std::sync::Arc;

use alloy_primitives::{Address, Bytes, TxKind};
use alloy_sol_types::{SolCall, SolType};
use lib_reth::{
    ExecuteEvm,
    reth_libmdbx::{NodeClientSpec, RethNodeClient},
    traits::EthRevm
};
use reth_provider::BlockNumReader;
use revm::context::TxEnv;

#[derive(Clone)]
pub struct RethDbProviderWrapper<N: NodeClientSpec> {
    provider: Arc<RethNodeClient<N>>
}

impl<N: NodeClientSpec> RethDbProviderWrapper<N> {
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

pub(crate) fn reth_db_view_call<Node, IC>(
    provider: &RethNodeClient<Node>,
    block_number: Option<u64>,
    contract: Address,
    call: IC
) -> eyre::Result<Result<IC::Return, alloy_sol_types::Error>>
where
    Node: NodeClientSpec,
    IC: SolCall + Send
{
    let tx = TxEnv {
        kind: TxKind::Call(contract),
        data: call.abi_encode().into(),
        ..Default::default()
    };

    let block_number = if let Some(bn) = block_number {
        bn
    } else {
        provider.eth_db_provider().best_block_number()?
    };

    let mut evm = provider.make_empty_evm(block_number)?;

    let data = evm.transact(tx)?;

    Ok(IC::abi_decode_returns(data.result.output().unwrap_or_default()))
}

pub(crate) fn reth_db_deploy_call<Node, IC>(
    provider: &RethNodeClient<Node>,
    block_number: Option<u64>,
    call_data: Bytes
) -> eyre::Result<Result<IC::RustType, alloy_sol_types::Error>>
where
    Node: NodeClientSpec,
    IC: SolType + Send
{
    let tx = TxEnv { kind: TxKind::Create, data: call_data, ..Default::default() };

    let block_number = if let Some(bn) = block_number {
        bn
    } else {
        provider.eth_db_provider().best_block_number()?
    };

    let mut evm = provider.make_empty_evm(block_number)?;

    let data = evm.transact(tx)?;

    Ok(IC::abi_decode(data.result.output().unwrap_or_default()))
}
