use crate::EthProvider;
use alloy_primitives::{Address, U256};

use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;

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
