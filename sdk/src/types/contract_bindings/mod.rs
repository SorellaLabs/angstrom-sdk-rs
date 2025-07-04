pub use user_position_fetcher::*;
#[rustfmt::skip]
mod user_position_fetcher {
    alloy_sol_types::sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        #[derive(Debug, PartialEq, Eq, Hash)]
        UserPositionFetcher,
        "../contracts/out/UserPositionFetcher.sol/UserPositionFetcher.json"
    );
}
