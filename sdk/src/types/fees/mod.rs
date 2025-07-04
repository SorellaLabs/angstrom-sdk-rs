use alloy_eips::BlockId;
use alloy_network::Ethereum;
use alloy_primitives::{Address, StorageKey, StorageValue};
use alloy_provider::{Provider, RootProvider};
use revm_database::{AlloyDB, async_db::DatabaseAsyncRef};

pub mod angstrom;
pub mod uniswap;

#[async_trait::async_trait]
pub trait StorageSlotFetcher {
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

mod reth_db_impls {
    use lib_reth::{EthApiServer, reth_libmdbx::RethLibmdbxClient, traits::EthRevm};
    use revm::DatabaseRef;

    use super::*;
    use crate::providers::local_reth::RethDbProviderWrapper;

    #[async_trait::async_trait]
    impl StorageSlotFetcher for RethLibmdbxClient {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_number: Option<u64>
        ) -> eyre::Result<StorageValue> {
            let block_number =
                if let Some(bn) = block_number { bn } else { self.eth_api().block_number()?.to() };

            let db = self.make_inner_db(block_number)?;
            Ok(db.storage_ref(address, key.into())?)
        }
    }

    #[async_trait::async_trait]
    impl<P: Provider + Clone> StorageSlotFetcher for RethDbProviderWrapper<P> {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_number: Option<u64>
        ) -> eyre::Result<StorageValue> {
            let db_client = self.db_client();
            let block_number = if let Some(bn) = block_number {
                bn
            } else {
                db_client.eth_api().block_number()?.to()
            };

            let db = db_client.make_inner_db(block_number)?;
            Ok(db.storage_ref(address, key.into())?)
        }
    }
}
