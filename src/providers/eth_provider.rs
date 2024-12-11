use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types::{Block, BlockTransactionsKind};
use alloy_sol_types::SolCall;

pub trait EthProvider: Clone + Send + 'static {
    async fn get_storage_at(&self, address: Address, key: U256) -> eyre::Result<U256>;

    async fn get_code_at(&self, address: Address) -> eyre::Result<Bytes>;

    // async fn get_erc20_info(
    //     &self,
    //     token_address: Address,
    // ) -> eyre::Result<TokenInfo>;

    async fn view_call<IC>(&self, contract: Address, call: IC) -> eyre::Result<IC::Return>
    where
        IC: SolCall + Send;

    async fn current_block_number(&self) -> eyre::Result<u64>;

    async fn get_block(&self, number: u64, kind: BlockTransactionsKind) -> eyre::Result<Block>;

    async fn get_nonce(&self, address: Address) -> eyre::Result<u64>;
}
