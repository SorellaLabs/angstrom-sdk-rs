use alloy_eips::BlockId;
use alloy_primitives::Address;
use alloy_provider::Provider;
use angstrom_types::{
    contract_payloads::angstrom::AngstromPoolConfigStore,
    sol_bindings::{RawPoolOrder, grouped_orders::AllOrders},
};

use crate::{
    FillWrapper,
    types::{ANGSTROM_ADDRESS, TransactionRequestWithLiquidityMeta},
};

pub async fn pool_config_store<P>(provider: &P) -> eyre::Result<AngstromPoolConfigStore>
where
    P: Provider,
{
    AngstromPoolConfigStore::load_from_chain(ANGSTROM_ADDRESS, BlockId::latest(), provider)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
}

pub(crate) trait FromAddress {
    fn from_address<F: FillWrapper>(&self, filler: &F) -> Address;
}

impl FromAddress for TransactionRequestWithLiquidityMeta {
    fn from_address<F: FillWrapper>(&self, filler: &F) -> Address {
        if let Some(a) = self.tx_request.from {
            a
        } else {
            filler.from().expect("expected `from` Address")
        }
    }
}

impl FromAddress for AllOrders {
    fn from_address<F: FillWrapper>(&self, filler: &F) -> Address {
        let order_from = self.from();
        if order_from == Address::default() {
            filler.from().expect("expected `from` Address")
        } else {
            order_from
        }
    }
}
