#[rustfmt::skip]
pub mod angstrom_l_2 {
    alloy_sol_types::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        AngstromL2,
        "../contracts/abis/AngstromL2.sol/AngstromL2.json"
    );
}
#[rustfmt::skip]
pub mod angstrom_l_2_factory {
    alloy_sol_types::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        AngstromL2Factory,
        "../contracts/abis/AngstromL2Factory.sol/AngstromL2Factory.json"
    );
}
