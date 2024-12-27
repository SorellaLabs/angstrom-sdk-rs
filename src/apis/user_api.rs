use alloy_primitives::Address;

pub trait AngstromUserApi {
    async fn get_positions(&self, user_address: Address) -> eyre::Result<()>;

    async fn get_pool_view(
        &self,
        user_address: Address,
        token0: Address,
        token1: Address
    ) -> eyre::Result<()>;
}
