#[cfg(feature = "local-reth")]
mod reth_db_impls {
    use alloy_network::Network;
    use alloy_primitives::{Address, StorageKey, StorageValue};
    use alloy_provider::Provider;
    use lib_reth::reth_libmdbx::NodeClientSpec;
    use uniswap_storage::StorageSlotFetcher;

    use crate::providers::local_reth::RethDbProviderWrapper;

    #[async_trait::async_trait]
    impl<Node, P, N> StorageSlotFetcher for RethDbProviderWrapper<Node, P, N>
    where
        Node: NodeClientSpec,
        N: Network,
        P: Provider<N> + Clone
    {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_number: Option<u64>
        ) -> eyre::Result<StorageValue> {
            Ok(self
                .provider()
                .storage_at(address, key.into(), block_number)
                .await?)
        }
    }
}
