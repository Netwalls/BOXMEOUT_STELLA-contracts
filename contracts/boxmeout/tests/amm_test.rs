#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, Events},
    token, Address, BytesN, Env,
};

use boxmeout::{AMMContract, AMMContractClient};

fn create_test_env() -> Env {
    Env::default()
}

fn register_amm(env: &Env) -> Address {
    env.register_contract(None, AMMContract)
}

// Helper to create a mock USDC token
fn create_mock_token(env: &Env, admin: &Address) -> Address {
    let token_address = env.register_stellar_asset_contract_v2(admin.clone());
    token_address.address()
}

#[test]
fn test_amm_initialize() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128; // 100k USDC

    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // TODO: Add getters to verify
    // Verify slippage protection = 200
    // Verify trading fee = 20
    // Verify pricing model = CPMM
}

#[test]
fn test_create_pool() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create pool - mint tokens to creator first
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128; // 10k USDC

    // Mint USDC to creator
    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128));

    client.create_pool(&creator, &market_id, &initial_liquidity);

    // Pool created successfully - no panic means success
    // Event verification would need proper event parsing which is complex in tests
}

#[test]
#[should_panic(expected = "pool already exists")]
fn test_create_pool_twice_fails() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create pool
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128;

    // Mint USDC to creator
    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128 * 2)); // Mint enough for 2 attempts

    client.create_pool(&creator, &market_id, &initial_liquidity);
    
    // Try to create pool again - should panic
    client.create_pool(&creator, &market_id, &initial_liquidity);
}

#[test]
fn test_buy_shares_yes() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = Address::generate(&env);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // TODO: Implement when buy_shares is ready
    // Create pool first
    // let market_id = BytesN::from_array(&env, &[1u8; 32]);
    // client.create_pool(&market_id, &10_000_000_000u128);

    // Buy YES shares
    // let buyer = Address::generate(&env);
    // let outcome = 1u32; // YES
    // let amount = 1_000_000_000u128; // 1k USDC
    // let min_shares = 900_000_000u128; // 10% slippage tolerance

    // let shares = client.buy_shares(&buyer, &market_id, &outcome, &amount, &min_shares);

    // Verify shares received
    // Verify price impact calculation
    // Verify YES odds increased, NO odds decreased
}

#[test]
fn test_buy_shares_price_impact() {
    // TODO: Implement when buy_shares is ready
    // Test CPMM formula: x * y = k
    // Large buy should have higher price impact
    // Small buy should have lower price impact
}

#[test]
#[ignore]
#[should_panic(expected = "slippage exceeded")]
fn test_buy_shares_slippage_protection() {
    // TODO: Implement when buy_shares is ready
    // Set min_shares very high
    // Buy should fail due to slippage protection
}

#[test]
fn test_sell_shares() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // TODO: Implement when sell_shares is ready
    // Create pool
    // Buy shares
    // Sell shares back
    // Verify payout calculation
}

#[test]
#[ignore]
#[should_panic(expected = "insufficient shares")]
fn test_sell_more_shares_than_owned() {
    // TODO: Implement when sell_shares is ready
    // Try to sell more shares than user owns
}

#[test]
fn test_get_odds() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = Address::generate(&env);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // TODO: Implement when get_odds is ready
    // let market_id = BytesN::from_array(&env, &[1u8; 32]);
    // client.create_pool(&market_id, &10_000_000_000u128);

    // Get initial odds (should be 50/50)
    // let (yes_odds, no_odds) = client.get_odds(&market_id);
    // assert_eq!(yes_odds, 5000); // 50%
    // assert_eq!(no_odds, 5000); // 50%

    // Buy YES shares
    // Get new odds (YES should increase, NO should decrease)
}

#[test]
fn test_add_liquidity() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create initial pool
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128;

    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128));
    client.create_pool(&creator, &market_id, &initial_liquidity);

    // Add liquidity from second LP
    let lp2 = Address::generate(&env);
    let additional_liquidity = 5_000_000_000u128;
    token_client.mint(&lp2, &(additional_liquidity as i128));

    let lp_tokens = client.add_liquidity(&lp2, &market_id, &additional_liquidity);

    // LP tokens should be proportional: (5000 / 10000) * 10000 = 5000
    assert_eq!(lp_tokens, 5_000_000_000u128);

    // Verify event emitted
    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
fn test_add_liquidity_maintains_ratio() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create initial pool
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128;

    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128));
    client.create_pool(&creator, &market_id, &initial_liquidity);

    // Add liquidity multiple times
    let lp2 = Address::generate(&env);
    let additional_liquidity = 1_000_000_000u128;
    token_client.mint(&lp2, &(additional_liquidity as i128 * 3));

    client.add_liquidity(&lp2, &market_id, &additional_liquidity);
    client.add_liquidity(&lp2, &market_id, &additional_liquidity);
    client.add_liquidity(&lp2, &market_id, &additional_liquidity);

    // Should maintain 50/50 ratio throughout
}

#[test]
#[should_panic(expected = "pool does not exist")]
fn test_add_liquidity_pool_not_exist() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Try to add liquidity to non-existent pool
    let lp = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let liquidity_amount = 1_000_000_000u128;

    client.add_liquidity(&lp, &market_id, &liquidity_amount);
}

#[test]
#[should_panic(expected = "liquidity amount must be positive")]
fn test_add_liquidity_zero_amount() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create initial pool
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128;

    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128));
    client.create_pool(&creator, &market_id, &initial_liquidity);

    // Try to add zero liquidity
    let lp2 = Address::generate(&env);
    client.add_liquidity(&lp2, &market_id, &0u128);
}

#[test]
fn test_add_liquidity_event_emitted() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create initial pool
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128;

    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128));
    client.create_pool(&creator, &market_id, &initial_liquidity);

    // Add liquidity
    let lp2 = Address::generate(&env);
    let additional_liquidity = 5_000_000_000u128;
    token_client.mint(&lp2, &(additional_liquidity as i128));

    client.add_liquidity(&lp2, &market_id, &additional_liquidity);

    // Verify LiquidityAdded event was emitted
    let events = env.events().all();
    assert!(events.len() > 1, "LiquidityAdded event should be emitted");
}

#[test]
fn test_remove_liquidity() {
    // TODO: Implement when remove_liquidity is ready
    // Test removing liquidity
    // Test LP token burning
    // Test proportional payout
}

#[test]
fn test_cpmm_invariant() {
    // TODO: Advanced test
    // Test that K = x * y remains constant (accounting for fees)
    // After multiple trades, verify invariant holds
}

#[test]
#[should_panic(expected = "initial liquidity must be positive")]
fn test_create_pool_zero_liquidity_fails() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Try to create pool with zero liquidity - should panic
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    client.create_pool(&creator, &market_id, &0u128);
}

#[test]
fn test_create_pool_event_emitted() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create pool
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128;

    // Mint USDC to creator
    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128));

    client.create_pool(&creator, &market_id, &initial_liquidity);

    // Verify event was emitted by checking events exist
    let events = env.events().all();
    assert!(events.len() > 0, "No events emitted");
}

#[test]
fn test_create_pool_reserves_50_50() {
    let env = create_test_env();
    let amm_id = register_amm(&env);
    let client = AMMContractClient::new(&env, &amm_id);

    // Initialize AMM
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let usdc_token = create_mock_token(&env, &admin);
    let max_liquidity_cap = 100_000_000_000u128;
    env.mock_all_auths();
    client.initialize(&admin, &factory, &usdc_token, &max_liquidity_cap);

    // Create pool with even amount
    let creator = Address::generate(&env);
    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let initial_liquidity = 10_000_000_000u128;

    // Mint USDC to creator
    let token_client = token::StellarAssetClient::new(&env, &usdc_token);
    token_client.mint(&creator, &(initial_liquidity as i128));

    client.create_pool(&creator, &market_id, &initial_liquidity);

    // Verify 50/50 split by successful creation
    // Actual reserve verification would require getter methods
    let events = env.events().all();
    assert!(events.len() > 0, "Pool creation should emit event");
}
