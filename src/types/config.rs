#[derive(Debug, Clone)]
pub struct AngstromApiConfig {
    eth_rpc_ip_or_domain: String,
    angstrom_ip_or_domain: String,
    angstrom_ws_port: Option<u64>,
    angstrom_http_port: Option<u64>,
    eth_http_port: Option<u64>,
    eth_ws_port: Option<u64>,
}

impl AngstromApiConfig {
    pub fn new(eth_rpc_ip_or_domain: impl ToString, angstrom_ip_or_domain: impl ToString) -> Self {
        Self {
            eth_rpc_ip_or_domain: eth_rpc_ip_or_domain.to_string(),
            angstrom_ip_or_domain: angstrom_ip_or_domain.to_string(),
            eth_http_port: None,
            eth_ws_port: None,
            angstrom_ws_port: None,
            angstrom_http_port: None,
        }
    }

    pub fn with_eth_ws_port(mut self, port: u64) -> Self {
        self.eth_ws_port = Some(port);
        self
    }

    pub fn with_eth_http_port(mut self, port: u64) -> Self {
        self.eth_http_port = Some(port);
        self
    }

    pub fn with_angstrom_ws_port(mut self, port: u64) -> Self {
        self.angstrom_ws_port = Some(port);
        self
    }

    pub fn with_angstrom_http_port(mut self, port: u64) -> Self {
        self.angstrom_http_port = Some(port);
        self
    }

    pub fn eth_ws_url(&self) -> String {
        format!(
            "ws://{}:{}",
            self.eth_rpc_ip_or_domain,
            self.eth_ws_port.expect("no eth_ws_port set")
        )
    }

    pub fn eth_http_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.eth_rpc_ip_or_domain,
            self.eth_http_port.expect("no eth_http_port set")
        )
    }

    pub fn angstrom_ws_url(&self) -> String {
        format!(
            "ws://{}:{}",
            self.angstrom_ip_or_domain,
            self.angstrom_ws_port.expect("no angstrom_ws_port set")
        )
    }

    pub fn angstrom_http_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.angstrom_ip_or_domain,
            self.angstrom_http_port.expect("no angstrom_http_port set")
        )
    }
}
