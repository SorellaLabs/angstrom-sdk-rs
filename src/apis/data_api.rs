use std::future::Future;

use crate::providers::EthProvider;
use alloy_primitives::U256;
use alloy_primitives::{Address, TxHash, B256};
use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;

use crate::types::*;

pub trait AngstromDataApi: EthProvider {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let value = self
            .get_storage_at(ANGSTROM_ADDRESS, U256::from(POOL_CONFIG_STORE_SLOT))
            .await?;

        let value_bytes: [u8; 32] = value.to_be_bytes();
        let config_store_address =
            Address::from(<[u8; 20]>::try_from(&value_bytes[4..24]).unwrap());

        let code = self.get_code_at(config_store_address).await?;

        let config_store = AngstromPoolConfigStore::try_from(code.0.to_vec().as_slice())
            .map_err(|e| eyre::eyre!("{e:?}"))?;

        Ok(vec![])
    }

    fn pool_metadata(
        &self,
        token0: Address,
        token1: Address,
    ) -> impl Future<Output = eyre::Result<PoolMetadata>> + Send;

    fn historical_trades(
        &self,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> impl Future<Output = eyre::Result<Vec<AllOrders>>> + Send;

    fn historical_trade(
        &self,
        tx_hash: TxHash,
        order_hash: B256,
    ) -> impl Future<Output = eyre::Result<AllOrders>> + Send;
}
