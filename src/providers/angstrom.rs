#[derive(Debug, Clone)]
pub struct AngstromProvider {
    rpc_client: reqwest::Client,
    rpc_url: String,
    http_port: u64,
    ws_port: u64,
}

pub struct AngstromProviderBuilder {
    rpc_url: String,
    http_port: u64,
    ws_port: u64,
    http_client: Option<reqwest::Client>,
}

impl AngstromProviderBuilder {
    pub fn new(rpc_url: String, http_port: u64, ws_port: u64) -> Self {
        Self {
            rpc_url,
            http_port,
            ws_port,
            http_client: None,
        }
    }

    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn build(self) -> AngstromProvider {
        AngstromProvider {
            rpc_client: self.http_client.unwrap_or_default(),
            rpc_url: self.rpc_url,
            http_port: self.http_port,
            ws_port: self.ws_port,
        }
    }
}
