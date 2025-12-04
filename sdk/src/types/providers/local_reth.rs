use std::sync::Arc;

use lib_reth::reth_libmdbx::{NodeClientSpec, RethNodeClient};

#[derive(Clone)]
pub struct RethDbProviderWrapper<N: NodeClientSpec> {
    provider: Arc<RethNodeClient<N>>
}

impl<N: NodeClientSpec> RethDbProviderWrapper<N> {
    pub fn new(provider: Arc<RethNodeClient<N>>) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> Arc<RethNodeClient<N>> {
        self.provider.clone()
    }

    pub fn provider_ref(&self) -> &RethNodeClient<N> {
        &self.provider
    }
}
