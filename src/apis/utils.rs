use alloy_eips::BlockId;
use alloy_provider::Provider;
use alloy_transport::Transport;
use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;

use crate::types::ANGSTROM_ADDRESS;

pub async fn pool_config_store<P>(provider: &P) -> eyre::Result<AngstromPoolConfigStore>
where
    P: Provider + Clone
{
    AngstromPoolConfigStore::load_from_chain(ANGSTROM_ADDRESS, BlockId::latest(), provider)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
}
