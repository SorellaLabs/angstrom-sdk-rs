pub mod api;
use std::marker::PhantomData;

use angstrom_types::primitive::AngstromAddressBuilder;
pub use api::AngstromApi;
pub(crate) mod backend;

use alloy_provider::Provider;

use crate::apis::node_api::AngstromOrderApiClient;

#[derive(Default)]
pub struct AngstromApiBuilder<P, T, F = ()>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    eth_provider:    Option<P>,
    angstrom_url:    &'static str,
    filler:          Option<F>,
    address_builder: AngstromAddressBuilder,
    _t:              PhantomData<fn() -> T>
}

impl<P, T, F> AngstromApiBuilder<P, T, F>
where
    P: Provider,
    T: AngstromOrderApiClient
{
}
