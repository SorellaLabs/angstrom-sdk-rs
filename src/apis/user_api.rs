use alloy_primitives::Address;
use angstrom_types::primitive::PoolId;

use super::data_api::AngstromDataApi;
use crate::types::UserLiquidityPosition;

pub trait AngstromUserApi: AngstromDataApi {
    async fn get_positions(
        &self,
        user_address: Address
    ) -> eyre::Result<Vec<UserLiquidityPosition>>;

    async fn get_positions_in_pool(
        &self,
        user_address: Address,
        token0: Address,
        token1: Address
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let all_positions = self.get_positions(user_address).await?;
        let pool_id = self.pool_id(token0, token1).await?;

        Ok(all_positions
            .into_iter()
            .filter(|position| PoolId::from(position.pool_key.clone()) == pool_id)
            .collect())
    }
}
