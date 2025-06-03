use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use alloy_eips::BlockId;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::{Address, TxKind, U256, keccak256};
use alloy_provider::{Provider, RootProvider, ext::AnvilApi};
use alloy_sol_types::{SolCall, SolValue};
use angstrom_types::{
    primitive::ERC20,
    sol_bindings::{
        grouped_orders::AllOrders,
        rpc_orders::{
            ExactFlashOrder, ExactStandingOrder, PartialFlashOrder, PartialStandingOrder,
            TopOfBlockOrder,
        },
    },
};
use jsonrpsee_http_client::HttpClient;
use revm::{
    Context, ExecuteEvm, MainBuilder,
    context::{BlockEnv, TxEnv},
    primitives::hardfork::SpecId,
};
use revm_database::{AlloyDB, CacheDB, EmptyDBTyped, WrapDatabaseAsync};
use rust_utils::ToHashMapByKey;
use testing_tools::order_generator::OrderGenerator;
use tokio::{runtime::Handle, sync::Notify};
use uniswap_v4::uniswap::pool_manager::{SyncedUniswapPools, TickRangeToLoad};

use crate::{
    apis::{
        AngstromNodeApi,
        data_api::AngstromDataApi,
        node_api::{AngstromOrderApiClient, AngstromOrderApiClientClone},
    },
    providers::backend::AngstromProvider,
    test_utils::{ANGSTROM_HTTP_URL, AlloyRpcProvider, ETH_WS_URL},
};

pub async fn make_order_generator<P, T>(
    provider: &AngstromProvider<P, T>,
) -> eyre::Result<(OrderGenerator<T>, tokio::sync::mpsc::Receiver<(TickRangeToLoad, Arc<Notify>)>)>
where
    P: Provider,
    T: AngstromOrderApiClientClone,
{
    let block_number = provider.eth_provider().get_block_number().await?;

    let uniswap_pools = provider
        .all_pool_data(Some(block_number))
        .await?
        .into_iter()
        .hashmap_by_key_val(|(_, pool)| (pool.public_address(), Arc::new(RwLock::new(pool))));
    let cloned = provider.angstrom_rpc_provider().clone();

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let generator = OrderGenerator::new(
        SyncedUniswapPools::new(Arc::new(uniswap_pools.into_iter().collect()), tx),
        block_number,
        cloned,
        20..50,
        0.5..0.7,
    );

    Ok((generator, rx))
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
            AllOrders::ExactFlash(order) => exact_flash = Some(order),
            AllOrders::PartialFlash(order) => partial_flash = Some(order),
            AllOrders::ExactStanding(order) => exact_standing = Some(order),
            AllOrders::PartialStanding(order) => partial_standing = Some(order),
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
        assert!(f(AllOrders::PartialFlash(self.partial_flash)).await, "partial_flash failed");
        assert!(f(AllOrders::ExactFlash(self.exact_flash)).await, "exact_flash failed");
        assert!(
            f(AllOrders::PartialStanding(self.partial_standing)).await,
            "partial_standing failed"
        );
        assert!(f(AllOrders::ExactStanding(self.exact_standing)).await, "exact_standing failed");
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
    pub provider: AngstromProvider<AlloyRpcProvider<RootProvider>, HttpClient>,
    handle: Handle,
    _anvil: AnvilInstance,
}

impl AnvilAngstromProvider {
    pub async fn new() -> eyre::Result<Self> {
        dotenv::dotenv().ok();
        let angstrom_http_url = std::env::var(ANGSTROM_HTTP_URL)
            .unwrap_or_else(|_| panic!("{ANGSTROM_HTTP_URL} not found in .env"));
        let eth_ws_url =
            std::env::var(ETH_WS_URL).unwrap_or_else(|_| panic!("{ETH_WS_URL} not found in .env"));

        let seed: u16 = rand::random();
        let eth_ipc = format!("/tmp/anvil_{seed}.ipc");
        let anvil = Anvil::new()
            .chain_id(11155111)
            .ipc_path(&eth_ipc)
            .fork(eth_ws_url)
            .try_spawn()?;

        let provider = AngstromProvider::new_angstrom_http(
            RootProvider::builder()
                .with_recommended_fillers()
                .connect(&eth_ipc)
                .await?,
            &angstrom_http_url,
        )?;

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

        let mut evm = Context::<BlockEnv>::new(EmptyDBTyped::default(), SpecId::default())
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
        let return_data = ERC20::balanceOfCall::abi_decode_returns(output)?;
        if return_data == U256::from(123456789) {
            return Ok(offset as u64);
        }
    }

    Err(eyre::eyre!("was not able to find balance offset"))
}
