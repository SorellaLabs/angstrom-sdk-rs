// macro_rules! reth_db_pool_tick_data_loader_impl {
//     ($($network_ext:ident),*) => {
//         $(
//             #[async_trait::async_trait]
//             impl<T: AllExtensions> PoolTickDataLoader<<$network_ext<T> as
// EthNetworkExt>::AlloyNetwork> for RethDbProviderWrapper<$network_ext<T>>
//             {
//                 async fn load_tick_data(
//                     &self,
//                     pool_id: PoolId,
//                     current_tick: I24,
//                     zero_for_one: bool,
//                     num_ticks: u16,
//                     tick_spacing: I24,
//                     pool_manager_address: Address,
//                     block_number: BlockId,
//                 ) -> eyre::Result<(Vec<TickData>, U256)> {
//                     __load_tick_data(
//                         self,
//                         pool_id,
//                         current_tick,
//                         zero_for_one,
//                         num_ticks,
//                         tick_spacing,
//                         pool_manager_address,
//                         block_number
//                     )
//                     .await
//                 }
//             }
//         )*

//     }
// }

// #[cfg(feature = "l1")]
// reth_db_pool_tick_data_loader_impl!(MainnetExt);
// #[cfg(feature = "l2")]
// reth_db_pool_tick_data_loader_impl!(BaseMainnetExt, UnichainMainnetExt);

// async fn __load_tick_data<N>(
//     this: &RethDbProviderWrapper<N>,
//     pool_id: PoolId,
//     current_tick: I24,
//     zero_for_one: bool,
//     num_ticks: u16,
//     tick_spacing: I24,
//     pool_manager_address: Address,
//     block_number: BlockId
// ) -> eyre::Result<(Vec<TickData>, U256)>
// where
//     N: EthNetworkExt,
//     N::RethNode: NodeClientSpec
// {
//     let deployer_tx = GetUniswapV4TickData::deploy_builder(
//         this.provider().alloy_root_provider().await?,
//         pool_id,
//         pool_manager_address,
//         zero_for_one,
//         current_tick,
//         num_ticks,
//         tick_spacing
//     )
//     .into_transaction_request();

//     let out_tick_data = this
//         .view_deploy_call::<TicksWithBlock>(block_number, deployer_tx)
//         .await?;

//     Ok((
//         out_tick_data
//             .ticks
//             .into_iter()
//             .take(out_tick_data.validTo.to::<usize>())
//             .collect::<Vec<_>>(),
//         out_tick_data.blockNumber
//     ))
// }
