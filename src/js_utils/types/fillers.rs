use alloy_signer_local::PrivateKeySigner;
use angstrom_sdk_macros::NeonObject;
use neon::object::Object;

#[derive(Clone, Debug, NeonObject)]
pub struct ClientFillerTypes {
    pub nonce:           bool,
    pub token_balance:   bool,
    pub signer:          bool,
    pub signer_priv_key: Option<PrivateKeySigner>
}
