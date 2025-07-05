use alloy_primitives::{
    Address, Bytes, TxKind, U256,
    aliases::{I24, U24}
};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::SolCall;
use angstrom_sdk_rs::{
    apis::AngstromOrderBuilder,
    builders::{
        _liquidity_calls, BurnPosition, DescreaseLiquidity, PositionManagerLiquidity,
        PositionManagerLiquidityBuilder, TakePair
    }
};
use angstrom_types::contract_bindings::pool_manager::PoolManager::PoolKey;

// Address of the PositionManager contract that manages NFT positions
// This contract wraps liquidity positions as NFTs and handles modifications
const POSITION_MANAGER: Address = Address::ZERO;

// Address of the Angstrom contract used as hooks for the pool
const ANGSTROM_CONTRACT: Address = Address::ZERO;

/// Decrease liquidity from an existing position using PositionManager
///
/// This function removes liquidity from an existing NFT position without
/// burning the NFT itself. The position remains active but with reduced
/// liquidity.
async fn decrease_liquidity_position_manager<P: Provider>(provider: P) -> eyre::Result<()> {
    // Define the pool where the position exists
    let pool_key = PoolKey {
        currency0:   Address::default(), // Token 0 address (lower address when sorted)
        currency1:   Address::default(), // Token 1 address (higher address when sorted)
        fee:         U24::from(0x800000), // Fee tier (0x800000 = 0.5% fee)
        tickSpacing: I24::default(),     // Tick spacing for the pool
        hooks:       ANGSTROM_CONTRACT   // Angstrom hooks contract
    };

    // Create a new builder for chaining multiple liquidity operations
    let mut builder = PositionManagerLiquidity::new();

    // Build the decrease liquidity operation and add it to the chain
    let decrease_liquidity = decrease_liquidity(&pool_key);
    builder.chain_builder(decrease_liquidity);

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

/// Build a decrease liquidity operation
///
/// Creates a builder that handles decreasing liquidity from an existing NFT
/// position and claiming the returned tokens.
fn decrease_liquidity(
    pool_key: &PoolKey
) -> PositionManagerLiquidityBuilder<DescreaseLiquidity, TakePair> {
    // Parameters for decreasing liquidity from an existing position
    let decrease_params = _liquidity_calls::decreaseLiquidityCall {
        tokenId:    U256::default(),  // NFT token ID of the position to decrease
        liquidity:  U256::default(),  // Amount of liquidity to remove
        amount0Min: u128::default(),  // Minimum amount of token0 to receive
        amount1Min: u128::default(),  // Minimum amount of token1 to receive
        hookData:   Bytes::default()  // Optional data for hooks
    };

    // Parameters for claiming the tokens after liquidity removal
    let take_pair_params = _liquidity_calls::takePairCall {
        currency0: pool_key.currency0, // First token to claim
        currency1: pool_key.currency1, // Second token to claim
        recipient: Address::default()  // Address to receive the tokens
    };

    // Build the decrease liquidity operation
    let mut builder = PositionManagerLiquidityBuilder::<DescreaseLiquidity>::new(decrease_params)
        .decrease_liquidity();

    // Add the take pair operation to claim the tokens
    builder.add_take_pair(take_pair_params);

    builder
}

/// Burn an entire position using PositionManager
///
/// This function completely removes a position by burning the NFT. All
/// liquidity is removed and the NFT is destroyed. This is different from
/// decrease_liquidity which keeps the NFT active.
async fn burn_position_manager<P: Provider>(provider: P) -> eyre::Result<()> {
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

    // Build the burn position operation and add it to the chain
    let burn_position = burn_position(&pool_key);
    builder.chain_builder(burn_position);

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

/// Build a burn position operation
///
/// Creates a builder that handles burning an NFT position completely,
/// removing all liquidity and destroying the NFT.
fn burn_position(pool_key: &PoolKey) -> PositionManagerLiquidityBuilder<BurnPosition, TakePair> {
    // Parameters for burning the position
    let burn_params = _liquidity_calls::burnPositionCall {
        tokenId:    U256::default(),  // NFT token ID of the position to burn
        amount0Min: u128::default(),  // Minimum amount of token0 to receive
        amount1Min: u128::default(),  // Minimum amount of token1 to receive
        hookData:   Bytes::default()  // Optional data for hooks
    };

    // Parameters for claiming all tokens after burning the position
    let take_pair_params = _liquidity_calls::takePairCall {
        currency0: pool_key.currency0, // First token to claim
        currency1: pool_key.currency1, // Second token to claim
        recipient: Address::default()  // Address to receive the tokens
    };

    // Build the burn position operation
    let mut builder =
        PositionManagerLiquidityBuilder::<BurnPosition>::new(burn_params).burn_position();

    // Add the take pair operation to claim all tokens
    builder.add_take_pair(take_pair_params);

    builder
}
