use alloy_provider::Provider;
use angstrom_sdk_macros::NeonObject;
use neon::object::Object;

use crate::{providers::AngstromProvider, types::fillers::FillWrapper};

#[derive(Clone, Debug, NeonObject)]
pub struct ClientFillerTypes {
    pub nonce:           bool,
    pub token_balance:   bool,
    pub signer:          bool,
    pub signer_priv_key: Option<String>
}
