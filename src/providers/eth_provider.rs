use std::future::Future;

use alloy_primitives::{Address, Bytes, U256};

pub trait EthProvider: Clone + Send + 'static {
    fn get_storage_at(
        &self,
        address: Address,
        key: U256,
    ) -> impl Future<Output = eyre::Result<U256>> + Send;

    fn get_code_at(&self, address: Address) -> impl Future<Output = eyre::Result<Bytes>> + Send;
}
