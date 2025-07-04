use alloy_primitives::{
    Address, Bytes, FixedBytes, I256, TxKind,
    aliases::{I24, U24}
};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::SolCall;
use angstrom_sdk_rs::apis::AngstromOrderBuilder;
use angstrom_types::contract_bindings::pool_manager::PoolManager::PoolKey;

// Address of the callback contract that will execute the liquidity modification
// This contract must implement the IModifyLiquidityCallback interface
const CALLBACK_CONTRACT: Address = Address::ZERO;

// Address of the Angstrom contract that will be set as the hooks address
// for the pool to enable Angstrom-specific features
const ANGSTROM_CONTRACT: Address = Address::ZERO;

/// Add liquidity directly through PoolManager using the callback pattern
///
/// This approach requires a deployed callback contract but provides full
/// control over the liquidity modification process. The callback contract
/// will handle token transfers and any additional logic needed during
/// the liquidity addition.
async fn add_liquidity_pool_manager<P: Provider>(provider: P) -> eyre::Result<()> {
    // Define the pool to add liquidity to
    // The pool is uniquely identified by its currency pair, fee tier, tick spacing,
    // and hooks
    let pool_key = PoolKey {
        currency0:   Address::default(), // Token 0 address (lower address when sorted)
        currency1:   Address::default(), // Token 1 address (higher address when sorted)
        fee:         U24::from(0x800000), // Dynamic fee tier
        tickSpacing: I24::default(),     /* Tick spacing for the pool (determines price
                                          * granularity) */
        hooks:       ANGSTROM_CONTRACT // Hooks contract address (Angstrom for enhanced features)
    };

    // Define the price range for the liquidity position
    let tick_lower = I24::default(); // Lower tick boundary of the position
    let tick_upper = I24::default(); // Upper tick boundary of the position

    // Amount of liquidity to add (positive value for adding liquidity)
    let liquidity_delta = I256::unchecked_from(100000);

    // Salt for position identification (allows multiple positions with same
    // parameters)
    let salt = FixedBytes::default();

    // Optional hook data to pass to the Angstrom hooks contract
    let hook_data = Bytes::default();

    // Build the modify liquidity call using the Angstrom SDK
    // This creates the calldata for the PoolManager's modifyLiquidity function
    let pool_manager_call = AngstromOrderBuilder::modify_liquidity(
        pool_key,
        tick_lower,
        tick_upper,
        liquidity_delta,
        hook_data,
        salt
    );

    // Create the transaction request
    // The transaction is sent to the callback contract, which will then
    // call the PoolManager's modifyLiquidity function
    let tx_request = TransactionRequest {
        to: Some(TxKind::Call(CALLBACK_CONTRACT)), // Send to callback contract
        input: pool_manager_call.abi_encode().into(), // Encoded function call as input
        ..Default::default()                       // Use default values for other fields
    };

    // Send the transaction and wait for confirmation
    provider.send_transaction(tx_request).await?.watch().await?;
    Ok(())
}
