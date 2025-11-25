mod local_reth;
mod root_provider;

use alloy_primitives::{BlockNumber, U256, aliases::I24};
use angstrom_types_primitives::PoolId;
pub use full::FullTickLoader;
use uni_v4::pool_data_loader::TickData;

pub const DEFAULT_TICKS_PER_BATCH: usize = 10;

#[async_trait::async_trait]
pub trait PoolTickDataLoader: Send + Sync {
    async fn load_tick_data(
        &self,
        pool_id: PoolId,
        current_tick: I24,
        zero_for_one: bool,
        num_ticks: u16,
        tick_spacing: I24,
        block_number: Option<BlockNumber>
    ) -> eyre::Result<(Vec<TickData>, U256)>;
}

mod full {
    use std::collections::HashMap;

    use alloy_primitives::{U256, aliases::I24};
    use angstrom_types_primitives::PoolId;
    use uni_v4::{
        baseline_pool_factory::BaselinePoolFactoryError, pool_data_loader::TickData,
        tick_info::TickInfo
    };

    use crate::utils::pool_tick_loaders::PoolTickDataLoader;

    fn flip_tick_if_not_init(tick_bitmap: &mut HashMap<i16, U256>, tick: i32, tick_spacing: i32) {
        let compressed = tick / tick_spacing;
        let word_pos = (compressed >> 8) as i16;
        let bit_pos = (compressed & 0xFF) as u8;

        let word = tick_bitmap.entry(word_pos).or_insert(U256::ZERO);
        let mask = U256::from(1) << bit_pos;

        if *word & mask == U256::ZERO {
            *word |= mask;
        }
    }

    #[async_trait::async_trait]
    pub trait FullTickLoader {
        async fn load_tick_data_in_band(
            &self,
            pool_id: PoolId,
            current_tick: i32,
            tick_spacing: i32,
            block_number: Option<u64>,
            tick_band: u16,
            ticks_per_batch: usize
        ) -> eyre::Result<(HashMap<i32, TickInfo>, HashMap<i16, U256>)>;

        async fn load_ticks_in_direction(
            &self,
            pool_id: PoolId,
            zero_for_one: bool,
            current_tick: i32,
            tick_spacing: i32,
            block_number: Option<u64>,
            tick_band: u16,
            ticks_per_batch: usize
        ) -> eyre::Result<Vec<TickData>>;

        async fn get_tick_data_batch_request(
            &self,
            pool_id: PoolId,
            tick_start: I24,
            zero_for_one: bool,
            num_ticks: u16,
            tick_spacing: i32,
            block_number: Option<u64>
        ) -> eyre::Result<(Vec<TickData>, i32)>;

        fn apply_ticks(
            &self,
            fetched_ticks: Vec<TickData>,
            tick_spacing: i32
        ) -> eyre::Result<(HashMap<i32, TickInfo>, HashMap<i16, U256>)>;
    }

    #[async_trait::async_trait]
    impl<T> FullTickLoader for T
    where
        T: PoolTickDataLoader + Sync
    {
        async fn load_tick_data_in_band(
            &self,
            pool_id: PoolId,
            current_tick: i32,
            tick_spacing: i32,
            block_number: Option<u64>,
            tick_band: u16,
            ticks_per_batch: usize
        ) -> eyre::Result<(HashMap<i32, TickInfo>, HashMap<i16, U256>)> {
            // Load ticks in both directions concurrently
            let (asks_result, bids_result) = futures::future::join(
                self.load_ticks_in_direction(
                    pool_id,
                    true,
                    current_tick,
                    tick_spacing,
                    block_number,
                    tick_band,
                    ticks_per_batch
                ),
                self.load_ticks_in_direction(
                    pool_id,
                    false,
                    current_tick,
                    tick_spacing,
                    block_number,
                    tick_band,
                    ticks_per_batch
                )
            )
            .await;

            let asks = asks_result?;
            let bids = bids_result?;

            // Combine tick data from both directions
            let mut all_ticks = asks;
            all_ticks.extend(bids);

            // Apply ticks to create final tick maps
            self.apply_ticks(all_ticks, tick_spacing)
        }

        async fn load_ticks_in_direction(
            &self,
            pool_id: PoolId,
            zero_for_one: bool,
            current_tick: i32,
            tick_spacing: i32,
            block_number: Option<u64>,
            tick_band: u16,
            ticks_per_batch: usize
        ) -> eyre::Result<Vec<TickData>> {
            let mut fetched_ticks = Vec::new();
            let mut tick_start = current_tick;
            let mut ticks_loaded = 0u16;

            while ticks_loaded < tick_band {
                let ticks_to_load = std::cmp::min(ticks_per_batch as u16, tick_band - ticks_loaded);

                let (batch_ticks, next_tick) = self
                    .get_tick_data_batch_request(
                        pool_id,
                        I24::unchecked_from(tick_start),
                        zero_for_one,
                        ticks_to_load,
                        tick_spacing,
                        block_number
                    )
                    .await?;

                fetched_ticks.extend(batch_ticks);
                ticks_loaded += ticks_to_load;

                // Update tick_start for next batch
                tick_start = if zero_for_one {
                    next_tick.wrapping_sub(tick_spacing)
                } else {
                    next_tick.wrapping_add(tick_spacing)
                };
            }

            Ok(fetched_ticks)
        }

        async fn get_tick_data_batch_request(
            &self,
            pool_id: PoolId,
            tick_start: I24,
            zero_for_one: bool,
            num_ticks: u16,
            tick_spacing: i32,
            block_number: Option<u64>
        ) -> eyre::Result<(Vec<TickData>, i32)> {
            let (ticks, _last_tick_bitmap) = self
                .load_tick_data(
                    pool_id,
                    tick_start,
                    zero_for_one,
                    num_ticks,
                    I24::unchecked_from(tick_spacing),
                    block_number
                )
                .await
                .map_err(|e| {
                    BaselinePoolFactoryError::PoolDataLoading(format!(
                        "Failed to load tick data: {e}"
                    ))
                })?;

            // Calculate next tick start position
            let next_tick = if zero_for_one {
                tick_start.as_i32() - (num_ticks as i32)
            } else {
                tick_start.as_i32() + (num_ticks as i32)
            };

            Ok((ticks, next_tick))
        }

        fn apply_ticks(
            &self,
            mut fetched_ticks: Vec<TickData>,
            tick_spacing: i32
        ) -> eyre::Result<(HashMap<i32, TickInfo>, HashMap<i16, U256>)> {
            let mut ticks = HashMap::new();
            let mut tick_bitmap = HashMap::new();

            // Sort ticks by tick value
            fetched_ticks.sort_by_key(|t| t.tick.as_i32());

            // Process only initialized ticks
            for tick_data in fetched_ticks.into_iter().filter(|t| t.initialized) {
                let tick_info = TickInfo {
                    initialized:   tick_data.initialized,
                    liquidity_net: tick_data.liquidityNet
                };

                ticks.insert(tick_data.tick.as_i32(), tick_info);

                // Update tick bitmap
                flip_tick_if_not_init(&mut tick_bitmap, tick_data.tick.as_i32(), tick_spacing);
            }

            Ok((ticks, tick_bitmap))
        }
    }
}
