use jsonrpsee_http_client::HttpClient;

use crate::apis::{node_api::AngstromNodeApi, order_builder::AngstromOrderBuilder};

#[derive(Debug, Clone)]
pub struct AngstromProvider {
    client: HttpClient,
}

impl AngstromProvider {
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }
}

impl AngstromNodeApi for AngstromProvider {
    fn rpc_provider(&self) -> HttpClient {
        self.client.clone()
    }
}

impl AngstromOrderBuilder for AngstromProvider {}
