use crate::EthProvider;
use alloy_primitives::{Address, U256};
use alloy_signer::{
    k256::ecdsa::{self, signature::hazmat::PrehashSigner, RecoveryId},
    SignerSync,
};
use alloy_signer_local::LocalSigner;
use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;
use angstrom_types::{
    primitive::ANGSTROM_DOMAIN,
    sol_bindings::rpc_orders::{OmitOrderMeta, OrderMeta},
};
use pade::PadeEncode;

use crate::types::{ANGSTROM_ADDRESS, POOL_CONFIG_STORE_SLOT};

pub async fn pool_config_store<E: EthProvider>(
    provider: &E,
) -> eyre::Result<AngstromPoolConfigStore> {
    let value = provider
        .get_storage_at(ANGSTROM_ADDRESS, U256::from(POOL_CONFIG_STORE_SLOT))
        .await?;

    let value_bytes: [u8; 32] = value.to_be_bytes();
    let config_store_address = Address::from(<[u8; 20]>::try_from(&value_bytes[4..24])?);

    let code = provider.get_code_at(config_store_address).await?;

    AngstromPoolConfigStore::try_from(code.0.to_vec().as_slice()).map_err(|e| eyre::eyre!("{e:?}"))
}

pub fn sign_into_meta<O: OmitOrderMeta, C: PrehashSigner<(ecdsa::Signature, RecoveryId)>>(
    signer: &LocalSigner<C>,
    order: &O,
) -> eyre::Result<OrderMeta> {
    let hash = order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
    let sig = signer.sign_hash_sync(&hash)?;
    Ok(OrderMeta {
        isEcdsa: true,
        from: signer.address(),
        signature: sig.pade_encode().into(),
    })
}
