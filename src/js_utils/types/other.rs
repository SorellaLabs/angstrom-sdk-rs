use alloy_primitives::Address;
use angstrom_sdk_rs_macros::NeonObject;
use neon::object::Object;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, NeonObject)]
pub struct TokenImageUrl {
    #[serde(skip)]
    pub address: Address,
    #[serde(rename = "logoURI")]
    pub url:     String
}

impl TokenImageUrl {
    pub fn set_address(mut self, address: Address) -> Self {
        self.address = address;
        self
    }
}
