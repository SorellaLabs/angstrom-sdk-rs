use angstrom_sdk_macros::NeonObject;

#[derive(Clone, Debug, NeonObject)]
pub struct ClientFillerTypes {
    nonce:           bool,
    token_balance:   bool,
    signer:          bool,
    signer_priv_key: String
}
