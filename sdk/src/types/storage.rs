#[cfg(feature = "local-reth")]
mod reth_db_impls {
    use alloy_primitives::{Address, StorageKey, StorageValue};
    use alloy_provider::Provider;
    use uniswap_storage::StorageSlotFetcher;

    use crate::providers::local_reth::RethDbProviderWrapper;

    #[async_trait::async_trait]
    impl<P: Provider + Clone> StorageSlotFetcher for RethDbProviderWrapper<P> {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_number: Option<u64>
        ) -> eyre::Result<StorageValue> {
            Ok(self
                .db_client()
                .eth_api()
                .storage_at(address, key.into(), block_number)
                .await?)
        }
    }
}
