#[derive(Debug, Clone)]
pub struct AngstromProvider {
    client: reqwest::Client,
    url: String,
}

impl AngstromProvider {
    pub fn new(angstrom_url: impl ToString) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: angstrom_url.to_string(),
        }
    }

    pub fn new_with_client(client: reqwest::Client, angstrom_url: impl ToString) -> Self {
        Self {
            client,
            url: angstrom_url.to_string(),
        }
    }
}
