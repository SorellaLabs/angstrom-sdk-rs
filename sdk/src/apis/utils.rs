use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_json_rpc::RpcError;
use alloy_network::Network;
use alloy_primitives::{Address, TxKind};
use alloy_provider::Provider;
use alloy_rpc_types::{Filter, TransactionInput, TransactionRequest};
use alloy_sol_types::{SolCall, SolEvent, SolType};
use alloy_transport::TransportErrorKind;
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager,
    primitive::{ANGSTROM_DEPLOYED_BLOCK, POOL_MANAGER_ADDRESS}
};

pub(crate) async fn view_call<P, IC>(
    provider: &P,
    block_number: Option<u64>,
    contract: Address,
    call: IC
) -> Result<Result<IC::Return, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
where
    P: Provider,
    IC: SolCall + Send
{
    let tx = TransactionRequest {
        to: Some(TxKind::Call(contract)),
        input: TransactionInput::both(call.abi_encode().into()),
        ..Default::default()
    };

    let data = provider
        .call(tx)
        .block(block_number.map(Into::into).unwrap_or(BlockId::latest()))
        .await?;
    Ok(IC::abi_decode_returns(&data))
}

pub(crate) async fn view_deploy<P, N, IC>(
    provider: &P,
    block_number: Option<u64>,
    tx: <N as Network>::TransactionRequest
) -> Result<Result<IC::RustType, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
where
    P: Provider<N>,
    N: Network,
    IC: SolType + Send
{
    let data = provider
        .call(tx)
        .block(block_number.map(Into::into).unwrap_or(BlockId::latest()))
        .await?;
    Ok(IC::abi_decode(&data))
}

pub(crate) fn historical_pool_manager_swap_filter(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Vec<Filter> {
    let swap_event = PoolManager::Swap::SIGNATURE_HASH;
    let pool_manager = *POOL_MANAGER_ADDRESS
        .get()
        .expect("POOL_MANAGER_ADDRESS has not been set");

    chunk_blocks(start_block, end_block)
        .into_iter()
        .map(|(s, e)| {
            Filter::new()
                .event_signature(swap_event)
                .address(pool_manager)
                .from_block(s)
                .to_block(e)
        })
        .collect()
}

pub(crate) fn historical_pool_manager_modify_liquidity_filter(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Vec<Filter> {
    let modify_liquidity_event = PoolManager::ModifyLiquidity::SIGNATURE_HASH;
    let pool_manager = *POOL_MANAGER_ADDRESS
        .get()
        .expect("POOL_MANAGER_ADDRESS has not been set");

    chunk_blocks(start_block, end_block)
        .into_iter()
        .map(|(s, e)| {
            Filter::new()
                .event_signature(modify_liquidity_event)
                .address(pool_manager)
                .from_block(s)
                .to_block(e)
        })
        .collect()
}

fn chunk_blocks(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Vec<(BlockNumberOrTag, BlockNumberOrTag)> {
    let mut start_block = start_block.unwrap_or_else(|| *ANGSTROM_DEPLOYED_BLOCK.get().unwrap());
    if let Some(eb) = end_block {
        let mut tags = Vec::new();
        while eb - start_block > 1000 {
            tags.push((start_block.into(), (start_block + 1000).into()));
            start_block += 1000;
        }
        tags.push((start_block.into(), eb.into()));
        tags
    } else {
        vec![(start_block.into(), BlockNumberOrTag::Latest)]
    }
}
