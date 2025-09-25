use alloy_eips::BlockId;
use alloy_network::Ethereum;
use alloy_primitives::{Address, StorageKey, StorageValue};
use alloy_provider::{Provider, RootProvider};
use auto_impl::auto_impl;
use revm_database::{AlloyDB, CacheDB, DatabaseRef, WrapDatabaseAsync, async_db::DatabaseAsyncRef};

#[async_trait::async_trait]
#[auto_impl(&, Box, Arc)]
pub trait StorageSlotFetcher: Sync {
    async fn storage_at(
        &self,
        address: Address,
        key: StorageKey,
        block_number: Option<u64>
    ) -> eyre::Result<StorageValue>;
}

#[async_trait::async_trait]
impl StorageSlotFetcher for RootProvider {
    async fn storage_at(
        &self,
        address: Address,
        key: StorageKey,
        block_number: Option<u64>
    ) -> eyre::Result<StorageValue> {
        Ok(self
            .get_storage_at(address, key.into())
            .block_id(block_number.map(Into::into).unwrap_or(BlockId::latest()))
            .await?)
    }
}

#[async_trait::async_trait]
impl StorageSlotFetcher for AlloyDB<Ethereum, RootProvider> {
    async fn storage_at(
        &self,
        address: Address,
        key: StorageKey,
        _: Option<u64>
    ) -> eyre::Result<StorageValue> {
        Ok(self.storage_async_ref(address, key.into()).await?)
    }
}

#[async_trait::async_trait]
impl<S: StorageSlotFetcher + DatabaseAsyncRef> StorageSlotFetcher for WrapDatabaseAsync<S> {
    async fn storage_at(
        &self,
        address: Address,
        key: StorageKey,
        _: Option<u64>
    ) -> eyre::Result<StorageValue> {
        self.storage_ref(address, key.into())
            .map_err(|e| eyre::eyre!("{e:?}"))
    }
}

#[async_trait::async_trait]
impl<S: StorageSlotFetcher + DatabaseRef> StorageSlotFetcher for CacheDB<S> {
    async fn storage_at(
        &self,
        address: Address,
        key: StorageKey,
        _: Option<u64>
    ) -> eyre::Result<StorageValue> {
        self.storage_ref(address, key.into())
            .map_err(|e| eyre::eyre!("{e:?}"))
    }
}

#[cfg(feature = "local-reth")]
mod reth_db_impls {
    use alloy_eips::BlockNumberOrTag;
    use reth_ethereum::rpc::{api::eth::RpcConvert, eth::RpcNodeCore};
    use reth_provider::{StateProvider, StateProviderFactory};

    use super::*;
    use crate::providers::local_reth::RethDbProviderWrapper;

    #[async_trait::async_trait]
    impl StorageSlotFetcher for dyn StateProvider {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_number: Option<u64>
        ) -> eyre::Result<StorageValue> {
            Ok(self.storage(address, key.into())?.ok_or_else(|| {
                eyre::eyre!(
                    "no storage found for block {block_number:?} at address {address:?} for key \
                     {key:?}"
                )
            })?)
        }
    }

    #[async_trait::async_trait]
    impl<N, Rpc> StorageSlotFetcher for RethDbProviderWrapper<N, Rpc>
    where
        N: RpcNodeCore,
        Rpc: RpcConvert
    {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_number: Option<u64>
        ) -> eyre::Result<StorageValue> {
            self.state_at(block_number)?
                .storage_at(address, key, block_number)
                .await
        }
    }

    // #[async_trait::async_trait]
    // impl StorageSlotFetcher for RethLibmdbxDatabaseRef {
    //     async fn storage_at(
    //         &self,
    //         address: Address,
    //         key: StorageKey,
    //         _: Option<u64>
    //     ) -> eyre::Result<StorageValue> {
    //         Ok(self.storage_ref(address, key.into())?)
    //     }
    // }
}
