use alloy_primitives::{
    Address, Bytes, FixedBytes, I256, TxKind, U256,
    aliases::{I24, U24}
};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::SolCall;
use angstrom_sdk_rs::{
    apis::AngstromOrderBuilder,
    builders::{
        _liquidity_calls, IncreaseLiquidity, MintPosition, PositionManagerLiquidity,
        PositionManagerLiquidityBuilder, SettlePair
    }
};
use angstrom_types::contract_bindings::pool_manager::PoolManager::PoolKey;

// Address of the PositionManager contract that manages NFT positions
// This contract wraps liquidity positions as NFTs and handles modifications
const POSITION_MANAGER: Address = Address::ZERO;

// Address of the Angstrom contract used as hooks for the pool
const ANGSTROM_CONTRACT: Address = Address::ZERO;

/// Increase liquidity for an existing position using PositionManager
///
/// This function adds more liquidity to an existing NFT position. The
/// PositionManager handles the complexity of tracking positions as NFTs and
/// ensures proper accounting.
async fn increase_liquidity_position_manager<P: Provider>(provider: P) -> eyre::Result<()> {
    // Define the pool where the position exists
    let pool_key = PoolKey {
        currency0:   Address::default(), // Token 0 address (lower address when sorted)
        currency1:   Address::default(), // Token 1 address (higher address when sorted)
        fee:         U24::from(0x800000), // Dynamic fee tier
        tickSpacing: I24::default(),     // Tick spacing for the pool
        hooks:       ANGSTROM_CONTRACT   // Angstrom hooks contract
    };

    // Create a new builder for chaining multiple liquidity operations
    let mut builder = PositionManagerLiquidity::new();

    // Build the increase liquidity operation and add it to the chain
    let increase_liquidity = increase_liquidity(&pool_key);
    builder.chain_builder(increase_liquidity);

    // Set deadline for the transaction (0 means no deadline)
    let deadline = U256::default();

    // Build the final call using the Angstrom SDK
    let position_manager_call = AngstromOrderBuilder::modify_liquidities(builder, deadline);

    // Create and send the transaction to the PositionManager
    let tx_request = TransactionRequest {
        to: Some(TxKind::Call(POSITION_MANAGER)), // Target the PositionManager contract
        input: position_manager_call.abi_encode().into(), // Encoded function call
        ..Default::default()
    };

    // Send transaction and wait for confirmation
    provider.send_transaction(tx_request).await?.watch().await?;
    Ok(())
}

/// Build an increase liquidity operation
///
/// Creates a builder that handles increasing liquidity for an existing NFT
/// position and settling the required token transfers.
fn increase_liquidity(
    pool_key: &PoolKey
) -> PositionManagerLiquidityBuilder<IncreaseLiquidity, SettlePair> {
    // Parameters for increasing liquidity on an existing position
    let increase_params = _liquidity_calls::increaseLiquidityCall {
        tokenId:    U256::default(),  // NFT token ID of the position to increase
        liquidity:  U256::default(),  // Amount of liquidity to add
        amount0Max: u128::default(),  // Maximum amount of token0 to spend
        amount1Max: u128::default(),  // Maximum amount of token1 to spend
        hookData:   Bytes::default()  // Optional data for hooks
    };

    // Parameters for settling the token transfers after liquidity increase
    let settle_params = _liquidity_calls::settlePairCall {
        currency0: pool_key.currency0, // First token to settle
        currency1: pool_key.currency1  // Second token to settle
    };

    // Build the increase liquidity operation with ERC20 token handling
    let mut builder = PositionManagerLiquidityBuilder::<IncreaseLiquidity>::new(increase_params)
        .increase_liquidity_with_erc20();

    // Add the settle operation to complete the token transfers
    builder.add_settle(settle_params);

    builder
}

/// Mint a new liquidity position using PositionManager
///
/// This function creates a new NFT position in the specified pool. The NFT
/// represents ownership of the liquidity position and can be transferred or
/// modified later.
async fn mint_position_manager<P: Provider>(provider: P) -> eyre::Result<()> {
    // Define the pool where the new position will be created
    let pool_key = PoolKey {
        currency0:   Address::default(), // Token 0 address (lower address when sorted)
        currency1:   Address::default(), // Token 1 address (higher address when sorted)
        fee:         U24::from(0x800000), // Fee tier (0x800000 = 0.5% fee)
        tickSpacing: I24::default(),     // Tick spacing for the pool
        hooks:       ANGSTROM_CONTRACT   // Angstrom hooks contract
    };

    // Create a new builder for chaining multiple liquidity operations
    let mut builder = PositionManagerLiquidity::new();

    // Build the mint position operation and add it to the chain
    let mint_position = mint_position(&pool_key);
    builder.chain_builder(mint_position);

    // Set deadline for the transaction (0 means no deadline)
    let deadline = U256::default();

    // Build the final call using the Angstrom SDK
    let position_manager_call = AngstromOrderBuilder::modify_liquidities(builder, deadline);

    // Create and send the transaction to the PositionManager
    let tx_request = TransactionRequest {
        to: Some(TxKind::Call(POSITION_MANAGER)), // Target the PositionManager contract
        input: position_manager_call.abi_encode().into(), // Encoded function call
        ..Default::default()
    };

    // Send transaction and wait for confirmation
    provider.send_transaction(tx_request).await?.watch().await?;
    Ok(())
}

/// Build a mint position operation
///
/// Creates a builder that handles minting a new NFT position with the specified
/// parameters and settling the required token transfers.
fn mint_position(pool_key: &PoolKey) -> PositionManagerLiquidityBuilder<MintPosition, SettlePair> {
    // Parameters for minting a new position
    let mint_params = _liquidity_calls::mintPositionCall {
        poolKey:    pool_key.clone().into(), // Pool to create position in
        tickLower:  I24::default(),          // Lower tick boundary of the position
        tickUpper:  I24::default(),          // Upper tick boundary of the position
        liquidity:  U256::default(),         // Initial liquidity amount
        amount0Max: u128::default(),         // Maximum amount of token0 to spend
        amount1Max: u128::default(),         // Maximum amount of token1 to spend
        owner:      Address::default(),      // Owner address for the new NFT
        hookData:   Bytes::default()         // Optional data for hooks
    };

    // Parameters for settling the token transfers after position creation
    let settle_params = _liquidity_calls::settlePairCall {
        currency0: pool_key.currency0, // First token to settle
        currency1: pool_key.currency1  // Second token to settle
    };

    // Build the mint position operation
    let mut builder =
        PositionManagerLiquidityBuilder::<MintPosition>::new(mint_params).mint_position();

    // Add the settle operation to complete the token transfers
    builder.add_settle(settle_params);

    builder
}
