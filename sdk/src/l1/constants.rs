use alloy_primitives::{Address, ChainId, address};
use alloy_sol_types::Eip712Domain;
use uniswap_storage::v4::{
    UNISWAP_V4_CONSTANTS_MAINNET, UNISWAP_V4_CONSTANTS_SEPOLIA_TESTNET, UniswapV4Constants
};

#[derive(Debug, Clone)]
pub struct AngstromL1Constants {
    angstrom_address:       Address,
    controller_v1_address:  Address,
    gas_token_address:      Address,
    angstrom_deploy_block:  u64,
    chain_id:               u64,
    angstrom_eip712_domain: Eip712Domain,
    uniswap_constants:      UniswapV4Constants
}

impl AngstromL1Constants {
    pub fn by_chain(chain_id: ChainId) -> Option<Self> {
        let _ = angstrom_types_primitives::primitive::try_init_with_chain_id(chain_id);
        match chain_id {
            1 => Some(ANGSTROM_L1_CONSTANTS_MAINNET),
            11155111 => Some(ANGSTROM_L1_CONSTANTS_SEPOLIA_TESTNET),
            _ => None
        }
    }

    #[inline]
    pub fn angstrom_address(&self) -> Address {
        self.angstrom_address
    }

    #[inline]
    pub fn controller_v1_address(&self) -> Address {
        self.controller_v1_address
    }

    #[inline]
    pub fn gas_token_address(&self) -> Address {
        self.gas_token_address
    }

    #[inline]
    pub fn angstrom_deploy_block(&self) -> u64 {
        self.angstrom_deploy_block
    }

    #[inline]
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    #[inline]
    pub fn angstrom_eip712_domain(&self) -> Eip712Domain {
        self.angstrom_eip712_domain.clone()
    }

    #[inline]
    pub fn uniswap_constants(&self) -> UniswapV4Constants {
        self.uniswap_constants
    }
}

pub const ANGSTROM_L1_CONSTANTS_MAINNET: AngstromL1Constants = AngstromL1Constants {
    angstrom_address:       address!("0x0000000aa232009084Bd71A5797d089AA4Edfad4"),
    controller_v1_address:  address!("0x1746484EA5e11C75e009252c102C8C33e0315fD4"),
    gas_token_address:      address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
    angstrom_deploy_block:  22971781,
    chain_id:               1,
    angstrom_eip712_domain: alloy_sol_types::eip712_domain!(
        name: "Angstrom",
        version: "v1",
        chain_id: 1,
        verifying_contract: address!("0x0000000aa232009084Bd71A5797d089AA4Edfad4"),
    ),
    uniswap_constants:      UNISWAP_V4_CONSTANTS_MAINNET
};

pub const ANGSTROM_L1_CONSTANTS_SEPOLIA_TESTNET: AngstromL1Constants = AngstromL1Constants {
    angstrom_address:       address!("0x3B9172ef12bd245A07DA0d43dE29e09036626AFC"),
    controller_v1_address:  address!("0x977c67e6CEe5b5De090006E87ADaFc99Ebed2a7A"),
    gas_token_address:      address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14"),
    angstrom_deploy_block:  8578780,
    chain_id:               11155111,
    angstrom_eip712_domain: alloy_sol_types::eip712_domain!(
        name: "Angstrom",
        version: "v1",
        chain_id: 11155111,
        verifying_contract: address!("0x3B9172ef12bd245A07DA0d43dE29e09036626AFC"),
    ),
    uniswap_constants:      UNISWAP_V4_CONSTANTS_SEPOLIA_TESTNET
};
