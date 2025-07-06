use alloy::transports::TransportErrorKind;
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_json_rpc::RpcError;
use alloy_primitives::{Address, TxKind};
use alloy_provider::Provider;
use alloy_rpc_types::{Filter, TransactionInput, TransactionRequest};
use alloy_sol_types::{SolCall, SolEvent};
use angstrom_types::{
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

pub(crate) fn historical_pool_manager_swap_filter(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Filter {
    let swap_event = PoolManager::Swap::SIGNATURE_HASH;
    let pool_manager = *POOL_MANAGER_ADDRESS
        .get()
        .expect("POOL_MANAGER_ADDRESS has not been set");

    let mut filter = Filter::new()
        .event_signature(swap_event)
        .address(pool_manager)
        .from_block(start_block.unwrap_or_else(|| *ANGSTROM_DEPLOYED_BLOCK.get().unwrap()))
        .to_block(
            end_block
                .map(Into::into)
                .unwrap_or_else(|| BlockNumberOrTag::Latest)
        );
    filter
}

pub(crate) fn historical_pool_manager_modify_liquidity_filter(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Filter {
    let modify_liquidity_event = PoolManager::ModifyLiquidity::SIGNATURE_HASH;
    let pool_manager = *POOL_MANAGER_ADDRESS
        .get()
        .expect("POOL_MANAGER_ADDRESS has not been set");

    let mut filter = Filter::new()
        .event_signature(modify_liquidity_event)
        .address(pool_manager)
        .from_block(start_block.unwrap_or_else(|| *ANGSTROM_DEPLOYED_BLOCK.get().unwrap()))
        .to_block(
            end_block
                .map(Into::into)
                .unwrap_or_else(|| BlockNumberOrTag::Latest)
        );

    filter
}
