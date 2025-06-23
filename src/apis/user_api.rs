use std::collections::{HashMap, HashSet};

use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::PoolKey, position_fetcher::PositionFetcher,
        position_manager::PositionManager
    },
    primitive::{POSITION_MANAGER_ADDRESS, PoolId}
};
use futures::TryFutureExt;

use super::{data_api::AngstromDataApi, utils::*};
use crate::types::UserLiquidityPosition;

pub trait AngstromUserApi: AngstromDataApi {
    async fn get_positions(
        &self,
        user_address: Address,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>>;

    async fn get_positions_in_pool(
        &self,
        user_address: Address,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let all_positions = self.get_positions(user_address, block_number).await?;
        let pool_id = self.pool_id(token0, token1, false, block_number).await?;

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
        _block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let user_positons = view_call(
            self,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            PositionFetcher::getPositionsCall {
                owner:       user_address,
                tokenId:     U256::from(1u8),
                lastTokenId: U256::ZERO,
                maxResults:  U256::MAX
            }
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
                    *POSITION_MANAGER_ADDRESS.get().unwrap(),
                    PositionManager::poolKeysCall { poolId: uni_id }
                )
                .and_then(async move |ang_id_res| {
                    Ok(ang_id_res.map(|ang_id| {
                        (
                            uni_id,
                            PoolKey {
                                currency0:   ang_id.currency0,
                                currency1:   ang_id.currency1,
                                fee:         ang_id.fee,
                                tickSpacing: ang_id.tickSpacing,
                                hooks:       ang_id.hooks
                            }
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
                    pos
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    // use alloy_primitives::address;

    // use super::*;
    // use crate::test_utils::spawn_angstrom_api;

    #[tokio::test]
    async fn test_get_positions() {
        // init_with_chain_id(11155111);
        // let angstrom_api = spawn_angstrom_api().await.unwrap();

        // let positions = angstrom_api
        //     .get_positions(address!("0xa7f1Aeb6e43443c683865Fdb9E15Dd01386C955b"))
        //     .await
        //     .unwrap();

        // println!("{positions:?}");

        todo!()
    }

    #[tokio::test]
    async fn test_get_positions_in_pool() {
        todo!()
    }
}
