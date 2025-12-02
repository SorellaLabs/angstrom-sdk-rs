use alloy_network::{Ethereum, Network};
use alloy_provider::Provider;

pub trait ProviderBlanket<N: Network = Ethereum>: Provider<N> + Clone {}

impl<P: Provider + Clone> ProviderBlanket for P {}
