use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use angstrom_types::contract_bindings::position_manager::PositionManager::PoolKey;

use crate::types::contract_bindings::UserPositionFetcher;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UserLiquidityPosition {
    pub token_id:   U256,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub liquidity:  u128,
    pub pool_key:   PoolKey
}

impl From<UserPositionFetcher::UserPosition> for UserLiquidityPosition {
    fn from(value: UserPositionFetcher::UserPosition) -> Self {
        Self {
            token_id:   value.tokenId,
            tick_lower: value.tickLower,
            tick_upper: value.tickUpper,
            liquidity:  value.liquidity,
            pool_key:   PoolKey {
                currency0:   value.poolKey.currency0,
                currency1:   value.poolKey.currency1,
                fee:         value.poolKey.fee,
                tickSpacing: value.poolKey.tickSpacing,
                hooks:       value.poolKey.hooks
            }
        }
    }
}

pub fn encode_angstrom_rewards_position_key(
    owner: Address,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> B256 {
    let mut bytes = [0u8; 70];
    bytes[12..32].copy_from_slice(&**owner);
    bytes[32..32 + 3].copy_from_slice(&tick_lower.to_be_bytes::<3>());
    bytes[32 + 3..32 + 6].copy_from_slice(&tick_upper.to_be_bytes::<3>());
    bytes[32 + 6..].copy_from_slice(&*B256::from(position_token_id));
    keccak256(&bytes[12..])
}

const TICK_LOWER_OFFSET: u32 = 8;
const TICK_UPPER_OFFSET: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnpackedPositionInfo {
    /// key for lookups in the PositionManager's `poolKeys` map
    pub position_manager_pool_map_key: [u8; 25],
    pub tick_lower:                    I24,
    pub tick_upper:                    I24
}

pub trait UnpackPositionInfo {
    fn unpack_position_info(&self) -> UnpackedPositionInfo;

    fn position_manager_pool_map_key(&self) -> [u8; 25];

    fn tick_lower(&self) -> I24;

    fn tick_upper(&self) -> I24;
}

impl UnpackPositionInfo for U256 {
    fn unpack_position_info(&self) -> UnpackedPositionInfo {
        UnpackedPositionInfo {
            position_manager_pool_map_key: self.position_manager_pool_map_key(),
            tick_lower:                    self.tick_lower(),
            tick_upper:                    self.tick_upper()
        }
    }

    fn position_manager_pool_map_key(&self) -> [u8; 25] {
        let shifted: U256 = *self >> 56;
        let mut out = [0u8; 25];
        out.copy_from_slice(&shifted.to_be_bytes_vec()[7..]);
        out
    }

    fn tick_lower(&self) -> I24 {
        let raw = ((*self >> TICK_LOWER_OFFSET) & U256::from((1u128 << 24) - 1)).to::<u32>();
        I24::unchecked_from(((raw << 8) as i32) >> 8)
    }

    fn tick_upper(&self) -> I24 {
        let raw = ((*self >> TICK_UPPER_OFFSET) & U256::from((1u128 << 24) - 1)).to::<u32>();
        I24::unchecked_from(((raw << 8) as i32) >> 8)
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::U256;

    use crate::{
        test_utils::valid_test_params::init_valid_position_params,
        types::positions::UnpackPositionInfo
    };

    #[test]
    fn test_unpack_position_info() {
        let pos_info = init_valid_position_params();

        let position_info_packed = U256::from_str_radix(
            "36752956352201235409813682138304141020772237719769761638105745524212318476800",
            10
        )
        .unwrap();

        let unpacked = position_info_packed.unpack_position_info();

        assert_eq!(unpacked.tick_lower, pos_info.tick_lower);
        assert_eq!(unpacked.tick_upper, pos_info.tick_upper);
        assert_eq!(unpacked.position_manager_pool_map_key, pos_info.position_manager_pool_map_key);
    }
}
