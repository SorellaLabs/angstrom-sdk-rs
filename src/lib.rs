#![allow(private_bounds)]
#![allow(async_fn_in_trait)]

pub mod apis;

pub mod providers;
#[cfg(test)]
pub mod test_utils;
pub mod types;

use std::collections::{HashMap, HashSet};

use alloy_network::TxSigner;
use alloy_primitives::{Address, FixedBytes, PrimitiveSignature, TxHash, U256};
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::PoolKey, position_fetcher::PositionFetcher,
        position_manager::PositionManager,
    },
    sol_bindings::{RawPoolOrder, grouped_orders::AllOrders},
};
use apis::{user_api::AngstromUserApi, utils::FromAddress};
use futures::TryFutureExt;
use jsonrpsee_http_client::HttpClient;
use providers::{AlloyWalletRpcProvider, AngstromProvider};
use types::{
    HistoricalOrders, HistoricalOrdersFilter, TokenInfoWithMeta, TokenPairInfo,
    TransactionRequestWithLiquidityMeta, UserLiquidityPosition,
    errors::AngstromSdkError,
    fillers::{
        AngstromFillProvider, AngstromFiller, FillWrapper, FillerOrderFrom, MakeFillerOrder,
        NonceGeneratorFiller, SignerFiller, TokenBalanceCheckFiller,
    },
};
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{data_api::AngstromDataApi, node_api::AngstromNodeApi},
    types::POSITION_MANAGER_ADDRESS,
};

#[derive(Clone)]
pub struct AngstromApi<P, F = ()>
where
    P: Provider,
{
    provider: AngstromProvider<P>,
    filler: F,
}

impl<P> AngstromApi<P>
where
    P: Provider,
{
    pub fn new(provider: AngstromProvider<P>) -> Self {
        Self { provider, filler: () }
    }

    pub fn eth_provider(&self) -> &P {
        self.provider.eth_provider()
    }

    pub fn angstrom_provider(&self) -> HttpClient {
        self.provider.angstrom_rpc_provider()
    }
}

impl<P, F> AngstromApi<P, F>
where
    P: Provider,
    F: FillWrapper,
{
    pub async fn send_add_remove_liquidity_tx(
        &self,
        tx_req: TransactionRequestWithLiquidityMeta,
    ) -> eyre::Result<TxHash> {
        let from = tx_req.from_address(&self.filler);
        let mut filled_tx_req = tx_req.convert_with_from(from);
        self.filler.fill(&self.provider, &mut filled_tx_req).await?;

        Ok(self
            .provider
            .send_add_remove_liquidity_tx(filled_tx_req.inner.force_regular_tx())
            .await?)
    }

    pub fn with_filler<F1: FillWrapper>(
        self,
        filler: F1,
    ) -> AngstromApi<P, AngstromFillProvider<F, F1>> {
        AngstromApi { provider: self.provider, filler: self.filler.wrap_with_filler(filler) }
    }

    pub fn with_nonce_generator_filler(
        self,
    ) -> AngstromApi<P, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            provider: self.provider,
            filler: self.filler.wrap_with_filler(NonceGeneratorFiller),
        }
    }

    pub fn with_token_balance_filler(
        self,
    ) -> AngstromApi<P, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            provider: self.provider,
            filler: self.filler.wrap_with_filler(TokenBalanceCheckFiller),
        }
    }

    pub fn with_signer_filler<S>(
        self,
        signer: S,
    ) -> AngstromApi<AlloyWalletRpcProvider<P>, AngstromFillProvider<F, SignerFiller<S>>>
    where
        S: Signer + SignerSync + TxSigner<PrimitiveSignature> + Clone + Send + Sync + 'static,
        SignerFiller<S>: AngstromFiller,
    {
        AngstromApi {
            provider: self.provider.with_wallet(signer.clone()),
            filler: self.filler.wrap_with_filler(SignerFiller::new(signer)),
        }
    }

    pub fn with_all_fillers<S>(
        self,
        signer: S,
    ) -> AngstromApi<
        P,
        AngstromFillProvider<
            AngstromFillProvider<
                AngstromFillProvider<F, NonceGeneratorFiller>,
                TokenBalanceCheckFiller,
            >,
            SignerFiller<S>,
        >,
    >
    where
        S: Signer + SignerSync + Send,
        SignerFiller<S>: AngstromFiller,
        P: Provider,
    {
        AngstromApi {
            provider: self.provider,
            filler: self
                .filler
                .wrap_with_filler(NonceGeneratorFiller)
                .wrap_with_filler(TokenBalanceCheckFiller)
                .wrap_with_filler(SignerFiller::new(signer)),
        }
    }
}

impl<P, F> AngstromNodeApi for AngstromApi<P, F>
where
    P: Provider,
    F: FillWrapper,
{
    fn angstrom_rpc_provider(&self) -> HttpClient {
        self.provider.angstrom_rpc_provider()
    }

    async fn send_order(&self, order: AllOrders) -> Result<FixedBytes<32>, AngstromSdkError> {
        let from = self.filler.from().unwrap_or_else(|| {
            let f = order.from();
            assert_ne!(f, Address::default());
            f
        });
        let mut filler_order: FillerOrderFrom = order.convert_with_from(from);
        self.filler.fill(&self.provider, &mut filler_order).await?;
        self.provider
            .send_order(filler_order.inner.force_all_orders())
            .await
    }

    async fn send_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> Result<Vec<Result<FixedBytes<32>, AngstromSdkError>>, AngstromSdkError> {
        let mut filler_orders: Vec<FillerOrderFrom> = orders
            .into_iter()
            .map(|o| {
                let from = o.from_address(&self.filler);
                o.convert_with_from(from)
            })
            .collect();

        self.filler
            .fill_many(&self.provider, &mut filler_orders)
            .await?;
        self.provider
            .send_orders(
                filler_orders
                    .into_iter()
                    .map(|order| order.inner.force_all_orders())
                    .collect(),
            )
            .await
    }
}

impl<P, F> AngstromDataApi for AngstromApi<P, F>
where
    P: Provider + Clone,
    F: FillWrapper,
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        self.provider.all_token_pairs().await
    }

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>> {
        self.provider.all_tokens().await
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.provider.historical_orders(filter).await
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader>> {
        self.provider.pool_data(token0, token1, block_number).await
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        self.provider.pool_key(token0, token1).await
    }
}

impl<P, F> AngstromUserApi for AngstromApi<P, F>
where
    P: Provider + Clone,
    F: FillWrapper,
{
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let user_positons = self
            .provider
            .view_call(
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
                self.provider
                    .view_call(
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

#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;
    use crate::test_utils::spawn_angstrom_api;

    #[tokio::test]
    async fn test_get_positions() {
        let angstrom_api = spawn_angstrom_api().await.unwrap();

        let positions = angstrom_api
            .get_positions(address!("0x796fB50EAe1456A523F869f6135dd557eeaEE226"))
            .await
            .unwrap();

        println!("{positions:?}");
    }
}
