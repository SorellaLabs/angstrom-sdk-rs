pub mod pool_tick_loaders;
pub mod providers;
pub(crate) mod utils;
use eth_network_exts::{
    AllExtensions, EthNetworkExt, base_mainnet::BaseMainnetExt, mainnet::MainnetExt,
    unichain_mainnet::UnichainMainnetExt
};

pub mod common;
pub mod fees;

pub mod contracts;

macro_rules! l2_network_ext_wrapper {
    ($($network_ext:ident),*) => {
        $(
            paste::paste!{
                #[derive(Clone)]
                pub struct [<$network_ext Wrapper>]<T = ()>(T);

                impl<T: AllExtensions> EthNetworkExt for [<$network_ext Wrapper>]<T> {
                    type AlloyNetwork = <$network_ext<T> as EthNetworkExt>::AlloyNetwork;
                    type RethNode = <$network_ext<T> as EthNetworkExt>::RethNode;
                    type TypeExt = <$network_ext<T> as EthNetworkExt>::TypeExt;

                    const CHAIN_ID: u64 = <$network_ext<T> as EthNetworkExt>::CHAIN_ID;
                }
            }
        )*
    };
}

#[cfg(feature = "l1")]
l2_network_ext_wrapper!(MainnetExt);

#[cfg(feature = "l2")]
l2_network_ext_wrapper!(BaseMainnetExt, UnichainMainnetExt);
