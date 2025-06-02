pub mod api;
use std::marker::PhantomData;

use angstrom_types::primitive::{AngstromAddressBuilder, init_with_chain_id};
pub use api::AngstromApi;
pub(crate) mod backend;

use alloy_provider::Provider;
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::WsClient;

use crate::apis::node_api::AngstromOrderApiClient;

#[derive(Default)]
pub struct AngstromApiBuilder<P, T, F = ()>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    eth_provider:    Option<P>,
    angstrom_url:    &'static str,
    address_builder: Option<AngstromAddressBuilder>,
    _t:              PhantomData<fn() -> (T, F)>
}

impl<P, T, F> AngstromApiBuilder<P, T, F>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    pub fn with_angstrom_addresses(self, address_builder: AngstromAddressBuilder) -> Self {
        Self { address_builder: Some(address_builder), ..self }
    }

    pub fn with_url(self, angstrom_url: &'static str) -> Self {
        Self { angstrom_url, ..self }
    }

    pub fn with_eth_provider(self, eth_provider: P) -> Self {
        Self { eth_provider: Some(eth_provider), ..self }
    }

    /// Uses the chain-id of the eth-provider if a address config is not set.
    pub async fn build_http(self) -> AngstromApi<P, HttpClient> {
        assert!(!self.angstrom_url.is_empty());
        let provider = self.eth_provider.expect("eth provider must be passed in");

        if let Some(address_builder) = self.address_builder {
            address_builder.build().try_init();
        } else {
            let chain_id = provider.get_chain_id().await.unwrap();
            init_with_chain_id(chain_id);
        }

        AngstromApi::new_angstrom_http(provider, self.angstrom_url).unwrap()
    }

    pub async fn build_ws(self) -> AngstromApi<P, WsClient> {
        assert!(!self.angstrom_url.is_empty());
        let provider = self.eth_provider.expect("eth provider must be passed in");

        if let Some(address_builder) = self.address_builder {
            address_builder.build().try_init();
        } else {
            let chain_id = provider.get_chain_id().await.unwrap();
            init_with_chain_id(chain_id);
        }

        AngstromApi::new_angstrom_ws(provider, self.angstrom_url)
            .await
            .unwrap()
    }
}
