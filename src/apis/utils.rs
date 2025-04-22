use alloy::transports::TransportErrorKind;
use alloy_eips::BlockId;
use alloy_json_rpc::RpcError;
use alloy_primitives::{Address, TxKind};
use alloy_provider::Provider;
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::SolCall;
use angstrom_types::{
    contract_payloads::angstrom::AngstromPoolConfigStore,
    sol_bindings::{RawPoolOrder, grouped_orders::AllOrders},
};

use crate::types::{ANGSTROM_ADDRESS, TransactionRequestWithLiquidityMeta, fillers::FillWrapper};

pub async fn pool_config_store<P>(provider: &P) -> eyre::Result<AngstromPoolConfigStore>
where
    P: Provider,
{
    AngstromPoolConfigStore::load_from_chain(ANGSTROM_ADDRESS, BlockId::latest(), provider)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
}

pub(crate) async fn view_call<P, IC>(
    provider: &P,
    contract: Address,
    call: IC,
) -> Result<Result<IC::Return, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
where
    P: Provider,
    IC: SolCall + Send,
{
    let tx = TransactionRequest {
        to: Some(TxKind::Call(contract)),
        input: TransactionInput::both(call.abi_encode().into()),
        ..Default::default()
    };

    let data = provider.call(tx).await?;
    Ok(IC::abi_decode_returns(&data))
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
