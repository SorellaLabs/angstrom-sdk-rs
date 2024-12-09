use std::future::Future;

use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types::{Block, BlockTransactionsKind};
use alloy_sol_types::SolCall;

pub trait EthProvider: Clone + Send + 'static {
    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> impl Future<Output = eyre::Result<U256>> + Send;

    fn get_code_at(&self, address: Address) -> impl Future<Output = eyre::Result<Bytes>> + Send;

    // fn get_erc20_info(
    //     &self,
    //     token_address: Address,
    // ) -> impl Future<Output = eyre::Result<TokenInfo>> + Send;

    fn view_call<IC>(
        &self,
        contract: Address,
        call: IC,
    ) -> impl Future<Output = eyre::Result<IC::Return>> + Send
    where
        IC: SolCall + Send;

    fn current_block_number(&self) -> impl Future<Output = eyre::Result<u64>> + Send;

    fn get_block(
        &self,
        number: u64,
        kind: BlockTransactionsKind,
    ) -> impl Future<Output = eyre::Result<Block>> + Send;
}
