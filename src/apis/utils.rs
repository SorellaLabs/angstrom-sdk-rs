use alloy_eips::BlockId;
use alloy_primitives::aliases::I24;
use alloy_primitives::I256;

use alloy_provider::Provider;
use alloy_transport::Transport;
use malachite::num::arithmetic::traits::Sign;
use std::cmp::Ordering;
use thiserror::Error;

use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;

use crate::types::ANGSTROM_ADDRESS;

const MIN_I24: i32 = -8_388_608_i32;
const MAX_I24: i32 = 8_388_607_i32;

pub async fn pool_config_store<P, T>(provider: &P) -> eyre::Result<AngstromPoolConfigStore>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    AngstromPoolConfigStore::load_from_chain(ANGSTROM_ADDRESS, BlockId::latest(), provider)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
}

pub fn i32_to_i24(val: i32) -> Result<I24, ConversionError> {
    if !(MIN_I24..=MAX_I24).contains(&val) {
        return Err(ConversionError::OverflowErrorI24(val));
    }
    let sign = val.sign();
    let inner = val.abs();

    let mut bytes = [0u8; 3];
    let value_bytes = inner.to_be_bytes();
    bytes[..].copy_from_slice(&value_bytes[1..]);

    let mut new = I24::from_be_bytes(bytes);
    if sign == Ordering::Less {
        new = -new;
    }
    Ok(new)
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("overflow from i32 to i24 {0:?}")]
    OverflowErrorI24(i32),
    #[error("overflow from I256 to I128 {0:?}")]
    OverflowErrorI28(I256),
}
