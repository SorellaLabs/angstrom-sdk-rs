use alloy_network::TransactionBuilder;
use alloy_primitives::{BlockNumber, U256, aliases::I24};
use angstrom_sdk_types::providers::local_reth::RethDbProviderWrapper;
use angstrom_types_primitives::{POOL_MANAGER_ADDRESS, PoolId};
use lib_reth::{EthereumNode, traits::EthStream};
use uni_v4::{
    loaders::get_uniswap_v_4_tick_data::GetUniswapV4TickData,
    pool_data_loader::{TickData, TicksWithBlock}
};

use crate::{
    providers::local_reth::reth_db_deploy_call, utils::pool_tick_loaders::PoolTickDataLoader
};

#[async_trait::async_trait]
impl PoolTickDataLoader for RethDbProviderWrapper<EthereumNode> {
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
            self.provider().root_provider().await?,
            pool_id,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            zero_for_one,
            current_tick,
            num_ticks,
            tick_spacing
        )
        .into_transaction_request();

        let out_tick_data = reth_db_deploy_call::<EthereumNode, TicksWithBlock>(
            self.provider_ref(),
            block_number,
            TransactionBuilder::input(&deployer_tx)
                .cloned()
                .unwrap_or_default()
        )??;

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
