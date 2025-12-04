#[cfg(feature = "local-reth")]
mod reth_db_impls {
    use alloy_primitives::{Address, StorageKey, StorageValue};
    use lib_reth::EthereumNode;
    use uniswap_storage::StorageSlotFetcher;

    use crate::types::providers::RethDbProviderWrapper;

    #[async_trait::async_trait]
    impl StorageSlotFetcher for RethDbProviderWrapper<EthereumNode> {
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
