use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{BlockNumber, U256, aliases::I24};
use angstrom_types_primitives::{POOL_MANAGER_ADDRESS, PoolId};
use uni_v4::{
    loaders::get_uniswap_v_4_tick_data::GetUniswapV4TickData,
    pool_data_loader::{TickData, TicksWithBlock}
};

use crate::types::{
    pool_tick_loaders::PoolTickDataLoader,
    providers::{AlloyProviderWrapper, alloy_view_deploy}
};

#[async_trait::async_trait]
impl<N> PoolTickDataLoader<N> for AlloyProviderWrapper<N>
where
    N: Network
{
    async fn load_tick_data(
        &self,
        pool_id: PoolId,
        current_tick: I24,
        zero_for_one: bool,
        num_ticks: u16,
        tick_spacing: I24,
        block_number: Option<BlockNumber>
    ) -> eyre::Result<(Vec<TickData>, U256)> {
        let deployer_tx = GetUniswapV4TickData::deploy_builder(
            self.provider(),
            pool_id,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            zero_for_one,
            current_tick,
            num_ticks,
            tick_spacing
        )
        .into_transaction_request();

        let out_tick_data =
            alloy_view_deploy::<_, _, TicksWithBlock>(
                self.provider(),
                block_number.map(Into::into).unwrap_or_else(BlockId::latest),
                deployer_tx
            )
            .await??;

        Ok((
            out_tick_data
                .ticks
                .into_iter()
                .take(out_tick_data.validTo.to::<usize>())
                .collect::<Vec<_>>(),
            out_tick_data.blockNumber
        ))
    }
}
