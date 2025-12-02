use alloy_primitives::{BlockNumber, U256, aliases::I24};
use angstrom_types_primitives::{POOL_MANAGER_ADDRESS, PoolId};
use uni_v4::{
    loaders::get_uniswap_v_4_tick_data::GetUniswapV4TickData,
    pool_data_loader::{TickData, TicksWithBlock}
};

use crate::{
    apis::utils::view_deploy,
    utils::{pool_tick_loaders::PoolTickDataLoader, provider_blanket::ProviderBlanket}
};

#[async_trait::async_trait]
impl<P> PoolTickDataLoader for P
where
    P: ProviderBlanket + Clone
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
            self,
            pool_id,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            zero_for_one,
            current_tick,
            num_ticks,
            tick_spacing
        )
        .into_transaction_request();

        let out_tick_data =
            view_deploy::<_, _, TicksWithBlock>(&self, block_number, deployer_tx).await??;

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
