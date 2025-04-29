use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    AngstromApi,
    apis::{
        data_api::AngstromDataApi,
        node_api::{AngstromNodeApi, AngstromOrderApiClient},
        order_builder::AngstromOrderBuilder,
    },
    types::fillers::AngstromFiller,
};
use alloy::{
    primitives::{Address, I256, U256},
    sol_types::SolCall,
    transports::TransportErrorKind,
};
use alloy_json_rpc::RpcError;
use alloy_primitives::TxKind;
use alloy_provider::Provider;
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use angstrom_types::{
    matching::{Ray, SqrtPriceX96},
    sol_bindings::grouped_orders::AllOrders,
};

pub struct ValidOrderGenerator<P: Provider, T: AngstromOrderApiClient, F: AngstromFiller> {
    angstrom_api: AngstromApi<P, T, F>,
}

impl<P: Provider, T: AngstromOrderApiClient, F: AngstromFiller> ValidOrderGenerator<P, T, F> {
    pub fn new(provider: AngstromApi<P, T, F>) -> Self {
        Self { angstrom_api: provider }
    }

    pub async fn generate_valid_tob_order(
        &self,
        token0: Address,
        token1: Address,
    ) -> eyre::Result<AllOrders> {
        let (block_number, pool) = self.angstrom_api.pool_data(token0, token1, None).await?;

        let pool_price = Ray::from(SqrtPriceX96::from(pool.sqrt_price));
        let mut gas = self
            .angstrom_api
            .estimate_gas(false, pool.token0, pool.token1)
            .await?;
        // cannot have zero gas.
        if gas.is_zero() {
            gas += U256::from(1);
        }

        let (amount, zfo) = self
            .fetch_direction_and_amounts(
                self.angstrom_api.from_address().unwrap(),
                token0,
                token1,
                &pool_price,
                true,
            )
            .await?;

        // limit to crossing 30 ticks a swap
        let target_price = if zfo {
            uniswap_v3_math::tick_math::get_sqrt_ratio_at_tick(pool.tick - (5 * pool.tick_spacing))
                .unwrap()
        } else {
            uniswap_v3_math::tick_math::get_sqrt_ratio_at_tick(pool.tick + (5 * pool.tick_spacing))
                .unwrap()
        };

        let t_in = if zfo { pool.token0 } else { pool.token1 };
        let (amount_in, amount_out) = pool
            .simulate_swap(t_in, amount, Some(target_price))
            .unwrap();

        let mut amount_in = u128::try_from(amount_in.abs()).unwrap();
        let mut amount_out = u128::try_from(amount_out.abs()).unwrap();

        if !zfo {
            std::mem::swap(&mut amount_in, &mut amount_out);
        }
        let range = (amount_in / 100).max(101);
        amount_in += self.gen_range_for(100, range);

        let (token_in, token_out) = if zfo { (token0, token1) } else { (token1, token0) };

        let order = AngstromOrderBuilder::tob_order(move |builder| {
            builder
                .asset_in(token_in)
                .asset_out(token_out)
                .quantity_in(amount_in)
                .max_gas(gas.to())
                .quantity_out(amount_out)
                .valid_block(block_number + 1)
                .build()
        });

        Ok(order)
    }

    // (amount, zfo)
    async fn fetch_direction_and_amounts(
        &self,
        from: Address,
        token0: Address,
        token1: Address,
        pool_price: &Ray,
        exact_in: bool,
    ) -> eyre::Result<(I256, bool)> {
        let (token0_bal, token1_bal) = tokio::try_join!(
            self.view_call(token0, _private::balanceOfCall::new((from,))),
            self.view_call(token1, _private::balanceOfCall::new((from,)))
        )?;

        let token0_bal = token0_bal?;
        let token1_bal = token1_bal?;

        if token0_bal.is_zero() || token1_bal.is_zero() {
            panic!(
                "no funds are in the given wallet t0: {:?} t1: {:?} wallet: {:?}",
                token0, token1, from
            );
        }

        let t1_with_current_price = pool_price.mul_quantity(token0_bal);
        // if the current amount of t0 mulled through the price is more than our other
        // balance this means that we have more t0 then t1 and thus want to sell
        // some t0 for t1
        let zfo = t1_with_current_price > token1_bal;

        let amount = if exact_in {
            // exact in will swap 1/6 of the balance
            I256::unchecked_from(if zfo {
                token0_bal / U256::from(50)
            } else {
                token1_bal / U256::from(50)
            })
        } else {
            // exact out
            I256::unchecked_from(if zfo {
                t1_with_current_price / U256::from(50)
            } else {
                token1_bal / U256::from(50)
            })
            .wrapping_neg()
        };

        Ok((amount, zfo))
    }

    fn gen_range_for(&self, lower: u128, upper: u128) -> u128 {
        assert!(lower < upper);
        let top = upper + 1;

        let modu = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % top;

        modu.max(lower)
    }

    async fn view_call<IC>(
        &self,
        contract: Address,
        call: IC,
    ) -> Result<Result<IC::Return, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
    where
        IC: SolCall + Send,
    {
        let tx = TransactionRequest {
            to: Some(TxKind::Call(contract)),
            input: TransactionInput::both(call.abi_encode().into()),
            ..Default::default()
        };

        let data = self.angstrom_api.eth_provider().call(tx).await?;
        Ok(IC::abi_decode_returns(&data))
    }
}

mod _private {
    use alloy::sol;

    sol! {
        function balanceOf(address owner) public view returns (uint256 result);
    }
}

#[allow(dead_code)]
fn main() {}
