use jsonrpsee_http_client::HttpClient;

use crate::apis::{node_api::AngstromNodeApi, order_builder::AngstromOrderBuilder};

#[derive(Debug, Clone)]
pub struct AngstromProvider {
    client: HttpClient,
}

impl AngstromProvider {
    pub fn new(rpc_url: impl ToString) -> eyre::Result<Self> {
        let client = HttpClient::builder().build(rpc_url.to_string())?;
        Ok(Self { client })
    }
}

impl AngstromNodeApi for AngstromProvider {
    fn rpc_provider(&self) -> HttpClient {
        self.client.clone()
    }
}

impl AngstromOrderBuilder for AngstromProvider {}
