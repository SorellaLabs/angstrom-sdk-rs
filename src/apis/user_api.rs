use alloy_primitives::Address;

use crate::providers::EthProvider;

pub trait AngstromUserApi {
    type EthProvider: EthProvider;

    async fn get_positions(&self, user_address: Address) -> eyre::Result<()>;

    async fn get_pool_view(
        &self,
        user_address: Address,
        token0: Address,
        token1: Address,
    ) -> eyre::Result<()>;
}
