#[derive(Debug, Clone)]
pub struct AngstromApiConfig {
    rpc_url: String,
    http_port: u64,
    ws_port: u64,
    angstrom_url: String,
}

impl AngstromApiConfig {
    pub fn new(
        rpc_url: impl ToString,
        angstrom_url: impl ToString,
        http_port: u64,
        ws_port: u64,
    ) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            http_port,
            ws_port,
            angstrom_url: angstrom_url.to_string(), // http_client: None,
        }
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}:{}", self.rpc_url, self.ws_port)
    }

    // pub fn with_client(mut self, client: reqwest::Client) -> Self {
    //     self.http_client = Some(client);
    //     self
    // }
}
