#[cfg(feature = "local-reth")]
mod reth_db_impls {
    use alloy_eips::BlockId;
    use alloy_primitives::{Address, StorageKey, StorageValue};
    use eth_network_exts::EthNetworkExt;
    use lib_reth::reth_libmdbx::NodeClientSpec;
    use reth_provider::StateProviderFactory;
    use uniswap_storage::StorageSlotFetcher;

    use crate::types::providers::RethDbProviderWrapper;

    #[async_trait::async_trait]
    impl<N> StorageSlotFetcher for RethDbProviderWrapper<N>
    where
        N: EthNetworkExt,
        N::RethNode: NodeClientSpec
    {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_id: BlockId
        ) -> eyre::Result<StorageValue> {
            let state_provider = self
                .provider_ref()
                .eth_db_provider()
                .state_by_block_id(block_id)?;

            Ok(state_provider.storage(address, key)?.unwrap_or_default())
        }
    }
}
