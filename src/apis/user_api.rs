use super::data_api::AngstromDataApi;
use super::utils::*;
use crate::types::{POSITION_MANAGER_ADDRESS, UserLiquidityPosition};
use alloy_primitives::Address;
use alloy_primitives::U256;
use alloy_provider::Provider;
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::PoolKey, position_fetcher::PositionFetcher,
        position_manager::PositionManager,
    },
    primitive::PoolId,
};
use auto_impl::auto_impl;
use futures::TryFutureExt;
use std::collections::HashMap;
use std::collections::HashSet;

pub trait AngstromUserApi: AngstromDataApi {
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>>;

    async fn get_positions_in_pool(
        &self,
        user_address: Address,
        token0: Address,
        token1: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let all_positions = self.get_positions(user_address).await?;
        let pool_id = self.pool_id(token0, token1).await?;

        Ok(all_positions
            .into_iter()
            .filter(|position| PoolId::from(position.pool_key.clone()) == pool_id)
            .collect())
    }
}

impl<P: Provider> AngstromUserApi for P {
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let user_positons = view_call(
            self,
            POSITION_MANAGER_ADDRESS,
            PositionFetcher::getPositionsCall {
                owner: user_address,
                tokenId: U256::from(1u8),
                lastTokenId: U256::ZERO,
                maxResults: U256::MAX,
            },
        )
        .await??;

        let unique_pool_ids = user_positons
            ._2
            .iter()
            .map(|pos: &PositionFetcher::Position| pos.poolId)
            .collect::<HashSet<_>>();

        let uni_pool_id_to_ang_pool_ids =
            futures::future::try_join_all(unique_pool_ids.into_iter().map(|uni_id| {
                view_call(
                    self,
                    POSITION_MANAGER_ADDRESS,
                    PositionManager::poolKeysCall { poolId: uni_id },
                )
                .and_then(async move |ang_id_res| {
                    Ok(ang_id_res.map(|ang_id| {
                        (
                            uni_id,
                            PoolKey {
                                currency0: ang_id.currency0,
                                currency1: ang_id.currency1,
                                fee: ang_id.fee,
                                tickSpacing: ang_id.tickSpacing,
                                hooks: ang_id.hooks,
                            },
                        )
                    }))
                })
            }))
            .await?
            .into_iter()
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(user_positons
            ._2
            .into_iter()
            .map(|pos| {
                UserLiquidityPosition::new(
                    uni_pool_id_to_ang_pool_ids
                        .get(&pos.poolId)
                        .unwrap()
                        .clone(),
                    pos,
                )
            })
            .collect())
    }
}
