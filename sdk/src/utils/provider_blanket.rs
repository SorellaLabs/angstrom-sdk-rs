use alloy_network::{Ethereum, Network};
use alloy_provider::Provider;

use crate::{
    AngstromApi, apis::AngstromOrderApiClient, providers::backend::AngstromProvider,
    types::fillers::AngstromFiller
};

pub trait ProviderBlanket<N: Network = Ethereum>: Provider<N> + Clone {}

impl<P, T, F> ProviderBlanket for AngstromApi<P, T, F>
where
    P: Provider + Clone,
    F: AngstromFiller,
    T: AngstromOrderApiClient + Clone
{
}

impl<P: Provider + Clone, T: AngstromOrderApiClient + Clone> ProviderBlanket
    for AngstromProvider<P, T>
{
}
