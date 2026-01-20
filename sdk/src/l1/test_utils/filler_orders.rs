use std::{fmt::Debug, sync::Arc};

use alloy_eips::BlockId;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::{Address, TxKind, U256, keccak256};
use alloy_provider::{Provider, RootProvider, ext::AnvilApi};
use alloy_sol_types::{SolCall, SolValue};
use angstrom_types_primitives::{
    primitive::ERC20,
    sol_bindings::{
        grouped_orders::AllOrders,
        rpc_orders::{
            ExactFlashOrder, ExactStandingOrder, PartialFlashOrder, PartialStandingOrder,
            TopOfBlockOrder
        }
    }
};
use jsonrpsee_http_client::HttpClient;
use revm::{
    Context, ExecuteEvm, MainBuilder,
    context::{BlockEnv, TxEnv},
    primitives::hardfork::SpecId
};
use revm_database::{AlloyDB, CacheDB, EmptyDBTyped, WrapDatabaseAsync};
#[cfg(feature = "example-utils")]
use testing_tools::order_generator::OrderGenerator;
use tokio::runtime::Handle;
#[cfg(feature = "example-utils")]
use uniswap_v4::uniswap::pool_manager::TickRangeToLoad;

#[cfg(feature = "example-utils")]
use crate::l1::test_utils::AngstromOrderApiClientClone;
use crate::l1::{providers::backend::AngstromProvider, test_utils::AlloyRpcProvider};

#[cfg(feature = "example-utils")]
pub async fn make_order_generator<P, T>(
    provider: &AngstromProvider<P, T>
) -> eyre::Result<(
    OrderGenerator<T>,
    tokio::sync::mpsc::Receiver<(TickRangeToLoad, Arc<tokio::sync::Notify>)>
)>
where
    P: Provider + Clone,
    T: AngstromOrderApiClientClone
{
    use alloy_primitives::aliases::U24;
    use angstrom_types_primitives::{
        PoolId,
        primitive::{POOL_MANAGER_ADDRESS, UniswapPoolRegistry}
    };
    use futures::future::try_join_all;
    use rust_utils::ToHashMapByKey;
    use testing_tools::order_generator::{InternalBalanceMode, OrderGenerator};
    use uni_v4::baseline_pool_factory::INITIAL_TICKS_PER_SIDE;
    use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_manager::SyncedUniswapPools};

    use crate::l1::apis::{AngstromNodeApi, data_api::AngstromL1DataApi};

    let block_number = provider.eth_provider().get_block_number().await?;

    let pools = provider.eth_provider().all_pool_keys(Some(block_number)).await?;

    let enhanced_pools = try_join_all(pools.into_iter().map(|pool_key| {
        let provider = provider.eth_provider().clone();
        async move {
            let public_pool_id: PoolId = pool_key.as_angstrom_pool_key_type().into();
            let mut private_pool_key = pool_key.pool_key;
            private_pool_key.fee = U24::from(0x800000);
            let private_pool_id: PoolId = private_pool_key.into();
            let registry = UniswapPoolRegistry::from(vec![pool_key.as_angstrom_pool_key_type()]);

            let data_loader = uniswap_v4::uniswap::pool_data_loader::DataLoader::new_with_registry(
                private_pool_id,
                public_pool_id,
                registry,
                *POOL_MANAGER_ADDRESS.get().unwrap()
            );

            let mut pool = EnhancedUniswapPool::new(data_loader, INITIAL_TICKS_PER_SIDE);
            pool.initialize(Some(block_number), Arc::new(provider))
                .await?;
            eyre::Ok((public_pool_id, Arc::new(std::sync::RwLock::new(pool))))
        }
    }))
    .await?
    .into_iter()
    .collect::<Vec<_>>();

    let uniswap_pools = enhanced_pools
        .into_iter()
        .hashmap_by_key_val(|(id, pool)| (id, pool));
    let cloned = provider.angstrom_rpc_provider().clone();

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let generator = OrderGenerator::new(
        SyncedUniswapPools::new(Arc::new(uniswap_pools.into_iter().collect()), tx),
        block_number,
        cloned,
        20..50,
        0.5..0.7,
        InternalBalanceMode::Random(0.5)
    );

    Ok((generator, rx))
}

#[derive(Debug, Clone, Default)]
pub struct AllOrdersSpecific {
    pub tob:              TopOfBlockOrder,
    pub partial_flash:    PartialFlashOrder,
    pub exact_flash:      ExactFlashOrder,
    pub partial_standing: PartialStandingOrder,
    pub exact_standing:   ExactStandingOrder
}

impl AllOrdersSpecific {
    pub fn with_address(&mut self, from: Address) {
        self.tob.meta.from = from;
        self.partial_flash.meta.from = from;
        self.exact_flash.meta.from = from;
        self.partial_standing.meta.from = from;
        self.exact_standing.meta.from = from;
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
    f: impl Fn(&AllOrders) -> Option<O>
) -> Option<(O, O)> {
    f(order0).zip(f(order1))
}

pub struct AnvilAngstromProvider {
    pub provider: AngstromProvider<AlloyRpcProvider<RootProvider>, HttpClient>,
    handle:       Handle,
    _anvil:       AnvilInstance
}

impl AnvilAngstromProvider {
    pub async fn new() -> eyre::Result<Self> {
        dotenv::dotenv().ok();
        let angstrom_http_url = std::env::var("ANGSTROM_HTTP_URL")
            .unwrap_or_else(|_| panic!("ANGSTROM_HTTP_URL not found in .env"));
        let eth_ws_url =
            std::env::var("ETH_WS_URL").unwrap_or_else(|_| panic!("ETH_WS_URL not found in .env"));

        let seed: u16 = rand::random();
        let eth_ipc = format!("/tmp/anvil_{seed}.ipc");
        let anvil = Anvil::new()
            .chain_id(1)
            .ipc_path(&eth_ipc)
            .fork(eth_ws_url)
            .try_spawn()?;

        let provider = AngstromProvider::new_angstrom_http(
            RootProvider::builder()
                .with_recommended_fillers()
                .connect(&eth_ipc)
                .await?,
            &angstrom_http_url
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

fn find_slot_offset_for_balance<P: Provider + Clone>(
    provider: &P,
    token_address: Address,
    handle: Handle
) -> eyre::Result<u64> {
    let probe_address = Address::random();

    let mut db = CacheDB::new(Arc::new(WrapDatabaseAsync::with_handle(
        AlloyDB::new(provider.root().clone(), BlockId::latest()),
        handle
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
