/// Add liquidity directly through PoolManager using the callback pattern
///
/// This approach requires a deployed callback contract but provides full
/// control over the liquidity modification process
async fn add_liquidity_direct(
    pool_manager: &PoolManager,
    pool_gate: &PoolGate, // Your callback contract
    pool_key: PoolKey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128
) -> Result<(u128, u128), Box<dyn Error>> {
    // Validate tick range before proceeding
    if tick_lower >= tick_upper {
        return Err("Invalid tick range: lower must be less than upper".into());
    }

    // Create modify liquidity parameters
    // The liquidityDelta is positive for adding liquidity, negative for removing
    let params = IPoolManager::ModifyLiquidityParams {
        tickLower:      I24::unchecked_from(tick_lower),
        tickUpper:      I24::unchecked_from(tick_upper),
        liquidityDelta: I256::from_raw(U256::from(liquidity_amount)), // Positive for adding
        salt:           [0u8; 32]                                     /* Can be used for
                                                                       * position identification
                                                                       * or left as zero */
    };

    println!("Adding liquidity with parameters:");
    println!("  Tick range: {} to {}", tick_lower, tick_upper);
    println!("  Liquidity amount: {}", liquidity_amount);
    println!("  Pool: {}/{}", pool_key.currency0, pool_key.currency1);

    // Execute through callback contract (PoolGate)
    // The callback contract handles the unlock pattern and settlement
    let tx = pool_gate
        .addLiquidity(
            pool_key.currency0,      // First token in the pair
            pool_key.currency1,      // Second token in the pair
            tick_lower.into(),       // Lower bound of position
            tick_upper.into(),       // Upper bound of position
            liquidity_amount.into(), // Amount of liquidity to add
            [0u8; 32]                // Salt for position identification
        )
        .send()
        .await?
        .watch() // Wait for transaction confirmation
        .await?;

    // Parse return values for actual token amounts used
    let receipt = provider.get_transaction_receipt(tx).await?;

    // Extract amounts from events or return data
    // These represent the actual amounts of token0 and token1 that were deposited
    let amount0 = extract_amount0(&receipt)?;
    let amount1 = extract_amount1(&receipt)?;

    println!("Successfully added liquidity:");
    println!("  Token0 amount: {}", amount0);
    println!("  Token1 amount: {}", amount1);

    Ok((amount0, amount1))
}
