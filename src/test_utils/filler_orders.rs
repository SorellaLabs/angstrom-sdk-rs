use crate::providers::backend::AlloyRpcProvider;
use crate::test_utils::{ANGSTROM_HTTP_URL, ETH_WS_URL};
use alloy_eips::BlockId;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::TxKind;
use alloy_primitives::{Address, U256, keccak256};
use alloy_provider::{Provider, RootProvider, ext::AnvilApi};
use alloy_sol_types::{SolCall, SolValue};
use angstrom_types::primitive::ERC20;
use angstrom_types::{
    CHAIN_ID,
    sol_bindings::{
        grouped_orders::{AllOrders, FlashVariants, GroupedVanillaOrder, StandingVariants},
        rpc_orders::{
            ExactFlashOrder, ExactStandingOrder, PartialFlashOrder, PartialStandingOrder,
            TopOfBlockOrder,
        },
    },
};
use revm::{
    Context,
    context::{BlockEnv, TxEnv},
    primitives::hardfork::SpecId,
};
use revm::{ExecuteEvm, MainBuilder};
use revm_database::{AlloyDB, CacheDB};
use revm_database::{EmptyDBTyped, WrapDatabaseAsync};
use rust_utils::ToHashMapByKey;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};
use testing_tools::order_generator::OrderGenerator;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use uniswap_v4::uniswap::pool_manager::{SyncedUniswapPools, TickRangeToLoad};

use crate::{apis::data_api::AngstromDataApi, providers::backend::AngstromProvider};

pub async fn make_order_generator<P>(
    provider: &AngstromProvider<P>,
) -> eyre::Result<(OrderGenerator, tokio::sync::mpsc::Receiver<(TickRangeToLoad, Arc<Notify>)>)>
where
    P: Provider + Clone,
{
    let block_number = provider.eth_provider().get_block_number().await?;

    let uniswap_pools = provider
        .all_pool_data(Some(block_number))
        .await?
        .into_iter()
        .hashmap_by_key_val(|pool| (pool.address(), Arc::new(RwLock::new(pool))));

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let generator = OrderGenerator::new(
        SyncedUniswapPools::new(Arc::new(uniswap_pools.into_iter().collect()), tx),
        block_number,
        20..50,
        0.5..0.7,
    );

    Ok((generator, rx))
}

pub fn generate_any_order_for_all(order_generator: &OrderGenerator) -> AllOrdersSpecific {
    let mut bitmap = 0x0000;
    let mut all_orders = Vec::new();

    loop {
        for order in order_generator.generate_orders() {
            if all_orders.is_empty() {
                all_orders.push(AllOrders::TOB(order.tob));
            }

            for book_order in &order.book {
                match book_order {
                    GroupedVanillaOrder::KillOrFill(FlashVariants::Partial(partial_flash)) => {
                        // check if bitmap contains a 1 in the first digit
                        if bitmap & 0b0001 != 0 {
                            all_orders.push(AllOrders::Flash(FlashVariants::Partial(
                                partial_flash.clone(),
                            )));
                            bitmap &= !0b0001;
                        }
                    }
                    GroupedVanillaOrder::KillOrFill(FlashVariants::Exact(exact_flash)) => {
                        // check if bitmap contains a 1 in the second digit
                        if bitmap & 0b0010 != 0 {
                            all_orders
                                .push(AllOrders::Flash(FlashVariants::Exact(exact_flash.clone())));
                            bitmap &= !0b0010;
                        }
                    }
                    GroupedVanillaOrder::Standing(StandingVariants::Partial(partial_standing)) => {
                        // check if bitmap contains a 1 in the third digit
                        if bitmap & 0b0100 != 0 {
                            all_orders.push(AllOrders::Standing(StandingVariants::Partial(
                                partial_standing.clone(),
                            )));
                            bitmap &= !0b0100;
                        }
                    }
                    GroupedVanillaOrder::Standing(StandingVariants::Exact(exact_standing)) => {
                        // check if bitmap contains a 1 in the fourth digit
                        if bitmap & 0b1000 != 0 {
                            all_orders.push(AllOrders::Standing(StandingVariants::Exact(
                                exact_standing.clone(),
                            )));
                            bitmap &= !0b1000;
                        }
                    }
                }
            }
        }

        if bitmap == 0x1111 {
            break AllOrdersSpecific::new(all_orders);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AllOrdersSpecific {
    pub tob: TopOfBlockOrder,
    pub partial_flash: PartialFlashOrder,
    pub exact_flash: ExactFlashOrder,
    pub partial_standing: PartialStandingOrder,
    pub exact_standing: ExactStandingOrder,
}

impl AllOrdersSpecific {
    fn new(orders: Vec<AllOrders>) -> Self {
        let mut tob: Option<TopOfBlockOrder> = None;
        let mut partial_flash: Option<PartialFlashOrder> = None;
        let mut exact_flash: Option<ExactFlashOrder> = None;
        let mut partial_standing: Option<PartialStandingOrder> = None;
        let mut exact_standing: Option<ExactStandingOrder> = None;

        orders.into_iter().for_each(|order| match order {
            AllOrders::Flash(FlashVariants::Exact(order)) => exact_flash = Some(order),
            AllOrders::Flash(FlashVariants::Partial(order)) => partial_flash = Some(order),
            AllOrders::Standing(StandingVariants::Exact(order)) => exact_standing = Some(order),
            AllOrders::Standing(StandingVariants::Partial(order)) => partial_standing = Some(order),
            AllOrders::TOB(order) => tob = Some(order),
        });

        Self {
            tob: tob.unwrap(),
            partial_flash: partial_flash.unwrap(),
            exact_flash: exact_flash.unwrap(),
            partial_standing: partial_standing.unwrap(),
            exact_standing: exact_standing.unwrap(),
        }
    }

    pub async fn test_filler_order(self, f: impl AsyncFn(AllOrders) -> bool) {
        assert!(f(AllOrders::TOB(self.tob)).await, "tob failed");
        assert!(
            f(AllOrders::Flash(FlashVariants::Partial(self.partial_flash))).await,
            "partial_flash failed"
        );
        assert!(
            f(AllOrders::Flash(FlashVariants::Exact(self.exact_flash))).await,
            "exact_flash failed"
        );
        assert!(
            f(AllOrders::Standing(StandingVariants::Partial(self.partial_standing))).await,
            "partial_standing failed"
        );
        assert!(
            f(AllOrders::Standing(StandingVariants::Exact(self.exact_standing))).await,
            "exact_standing failed"
        );
    }
}

pub fn match_all_orders<O>(
    order0: &AllOrders,
    order1: &AllOrders,
    f: impl Fn(&AllOrders) -> Option<O>,
) -> Option<(O, O)> {
    f(order0).zip(f(order1))
}

pub struct AnvilAngstromProvider {
    pub provider: AngstromProvider<AlloyRpcProvider<RootProvider>>,
    handle: Handle,
    _anvil: AnvilInstance,
}

impl AnvilAngstromProvider {
    pub async fn new() -> eyre::Result<Self> {
        dotenv::dotenv().ok();
        let angstrom_http_url = std::env::var(ANGSTROM_HTTP_URL)
            .expect(&format!("{ANGSTROM_HTTP_URL} not found in .env"));
        let eth_ws_url =
            std::env::var(ETH_WS_URL).expect(&format!("{ETH_WS_URL} not found in .env"));

        let seed: u16 = rand::random();
        let eth_ipc = format!("/tmp/anvil_{seed}.ipc");
        let anvil = Anvil::new()
            .chain_id(CHAIN_ID)
            .ipc_path(&eth_ipc)
            .fork(eth_ws_url)
            .try_spawn()?;

        let provider = AngstromProvider::new(&eth_ipc, &angstrom_http_url).await?;

        Ok(Self { provider, _anvil: anvil, handle: tokio::runtime::Handle::current().clone() })
    }

    pub async fn overwrite_token_amounts(&self, user: Address, token: Address) -> eyre::Result<()> {
        let balance_slot =
            find_slot_offset_for_balance(self.provider.eth_provider(), token, self.handle.clone())?;
        let owner_balance_slot = keccak256((user, balance_slot).abi_encode());

        self.provider
            .eth_provider()
            .anvil_set_storage_at(token, owner_balance_slot.into(), U256::MAX.into())
            .await?;

        Ok(())
    }
}

fn find_slot_offset_for_balance<P: Provider>(
    provider: &P,
    token_address: Address,
    handle: Handle,
) -> eyre::Result<u64> {
    let probe_address = Address::random();

    let mut db = CacheDB::new(Arc::new(WrapDatabaseAsync::with_handle(
        AlloyDB::new(provider.root().clone(), BlockId::latest()),
        handle,
    )));

    // check the first 100 offsets
    for offset in 0..100 {
        // set balance
        let balance_slot = keccak256((probe_address, offset as u64).abi_encode());
        db.insert_account_storage(token_address, balance_slot.into(), U256::from(123456789))?;
        // execute revm to see if we hit the slot

        let mut evm = Context::<BlockEnv>::new(EmptyDBTyped::default(), SpecId::LATEST)
            .with_ref_db(&db)
            .modify_cfg_chained(|cfg| {
                cfg.disable_balance_check = true;
            })
            .modify_tx_chained(|tx: &mut TxEnv| {
                tx.caller = probe_address;
                tx.kind = TxKind::Call(token_address);
                tx.data = ERC20::balanceOfCall::new((probe_address,))
                    .abi_encode()
                    .into();
                tx.value = U256::from(0);
            })
            .build_mainnet();

        let binding = evm.replay().map_err(|e| eyre::eyre!("{e:?}"))?;
        let Some(output) = binding.result.output() else {
            continue;
        };
        let return_data = ERC20::balanceOfCall::abi_decode_returns(output, false)?;
        if return_data.balance == U256::from(123456789) {
            return Ok(offset as u64);
        }
    }

    Err(eyre::eyre!("was not able to find balance offset"))
}
