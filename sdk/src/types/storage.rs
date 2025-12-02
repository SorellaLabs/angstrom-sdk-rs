#[cfg(feature = "local-reth")]
mod reth_db_impls {
    use alloy_primitives::{Address, StorageKey, StorageValue};
    use uniswap_storage::StorageSlotFetcher;

    use crate::providers::local_reth::RethDbProviderWrapper;

    #[async_trait::async_trait]
    impl StorageSlotFetcher for RethDbProviderWrapper {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_number: Option<u64>
        ) -> eyre::Result<StorageValue> {
            Ok(self
                .provider()
                .storage_at(address, key, block_number)
                .await?)
        }
    }
}
