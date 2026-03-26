// External integration tests for PredictionMarketContract::initialize (Issue #1)
// These mirror the pattern of the other *_test.rs files in this directory.
// The unit tests inside prediction_market.rs cover the same cases; these
// tests exercise the contract through the generated client, exactly as an
// external caller would.

#![cfg(test)]

use boxmeout::prediction_market::{
    PredictionMarketContract, PredictionMarketContractClient, PredictionMarketError,
};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (Env, Address, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let oracle = Address::generate(&env);
    let token = Address::generate(&env);
    let contract_id = env.register(PredictionMarketContract, ());
    (env, contract_id, admin, treasury, oracle, token)
}

fn init_default(
    env: &Env,
    cid: &Address,
    admin: &Address,
    treasury: &Address,
    oracle: &Address,
    token: &Address,
) -> Result<(), PredictionMarketError> {
    PredictionMarketContractClient::new(env, cid).try_initialize(
        admin, treasury, oracle, token,
        &200u32, &100u32,
        &1_000i128, &100i128, &2u32, &500i128,
    )
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_succeeds() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    assert!(init_default(&env, &cid, &admin, &treasury, &oracle, &token).is_ok());
}

#[test]
fn test_config_fields_stored_correctly() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    init_default(&env, &cid, &admin, &treasury, &oracle, &token).unwrap();

    let client = PredictionMarketContractClient::new(&env, &cid);
    let cfg = client.get_config().expect("config must be present");

    assert_eq!(cfg.admin, admin);
    assert_eq!(cfg.treasury, treasury);
    assert_eq!(cfg.oracle, oracle);
    assert_eq!(cfg.token, token);
    assert_eq!(cfg.protocol_fee_bps, 200);
    assert_eq!(cfg.creator_fee_bps, 100);
    assert_eq!(cfg.min_liquidity, 1_000);
    assert_eq!(cfg.min_trade, 100);
    assert_eq!(cfg.max_outcomes, 2);
    assert_eq!(cfg.dispute_bond, 500);
}

#[test]
fn test_next_market_id_is_one() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    init_default(&env, &cid, &admin, &treasury, &oracle, &token).unwrap();
    let client = PredictionMarketContractClient::new(&env, &cid);
    assert_eq!(client.get_next_market_id(), 1u64);
}

#[test]
fn test_emergency_pause_is_false() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    init_default(&env, &cid, &admin, &treasury, &oracle, &token).unwrap();
    let client = PredictionMarketContractClient::new(&env, &cid);
    assert!(!client.is_paused());
}

#[test]
fn test_initialized_event_emitted() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    init_default(&env, &cid, &admin, &treasury, &oracle, &token).unwrap();
    assert!(!env.events().all().is_empty());
}

// ---------------------------------------------------------------------------
// AlreadyInitialized guard
// ---------------------------------------------------------------------------

#[test]
fn test_second_call_rejected() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    init_default(&env, &cid, &admin, &treasury, &oracle, &token).unwrap();

    let result = init_default(&env, &cid, &admin, &treasury, &oracle, &token);
    assert_eq!(result, Err(Ok(PredictionMarketError::AlreadyInitialized)));
}

#[test]
fn test_second_call_does_not_mutate_config() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    init_default(&env, &cid, &admin, &treasury, &oracle, &token).unwrap();

    let client = PredictionMarketContractClient::new(&env, &cid);
    // Attempt with different fees – must be rejected
    let _ = client.try_initialize(
        &admin, &treasury, &oracle, &token,
        &9_000u32, &1_000u32,
        &1_000i128, &100i128, &2u32, &500i128,
    );

    let cfg = client.get_config().unwrap();
    assert_eq!(cfg.protocol_fee_bps, 200); // original value unchanged
}

// ---------------------------------------------------------------------------
// Fee validation
// ---------------------------------------------------------------------------

#[test]
fn test_fees_over_10000_rejected() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let client = PredictionMarketContractClient::new(&env, &cid);
    let result = client.try_initialize(
        &admin, &treasury, &oracle, &token,
        &9_000u32, &2_000u32,
        &1_000i128, &100i128, &2u32, &500i128,
    );
    assert_eq!(result, Err(Ok(PredictionMarketError::FeesTooHigh)));
}

#[test]
fn test_fees_exactly_10000_accepted() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let client = PredictionMarketContractClient::new(&env, &cid);
    let result = client.try_initialize(
        &admin, &treasury, &oracle, &token,
        &5_000u32, &5_000u32,
        &1_000i128, &100i128, &2u32, &500i128,
    );
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Limit validations
// ---------------------------------------------------------------------------

#[test]
fn test_zero_min_liquidity_rejected() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let result = PredictionMarketContractClient::new(&env, &cid).try_initialize(
        &admin, &treasury, &oracle, &token,
        &200u32, &100u32, &0i128, &100i128, &2u32, &500i128,
    );
    assert_eq!(result, Err(Ok(PredictionMarketError::InvalidMinLiquidity)));
}

#[test]
fn test_zero_min_trade_rejected() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let result = PredictionMarketContractClient::new(&env, &cid).try_initialize(
        &admin, &treasury, &oracle, &token,
        &200u32, &100u32, &1_000i128, &0i128, &2u32, &500i128,
    );
    assert_eq!(result, Err(Ok(PredictionMarketError::InvalidMinTrade)));
}

#[test]
fn test_max_outcomes_one_rejected() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let result = PredictionMarketContractClient::new(&env, &cid).try_initialize(
        &admin, &treasury, &oracle, &token,
        &200u32, &100u32, &1_000i128, &100i128, &1u32, &500i128,
    );
    assert_eq!(result, Err(Ok(PredictionMarketError::InvalidMaxOutcomes)));
}

#[test]
fn test_max_outcomes_257_rejected() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let result = PredictionMarketContractClient::new(&env, &cid).try_initialize(
        &admin, &treasury, &oracle, &token,
        &200u32, &100u32, &1_000i128, &100i128, &257u32, &500i128,
    );
    assert_eq!(result, Err(Ok(PredictionMarketError::InvalidMaxOutcomes)));
}

#[test]
fn test_max_outcomes_256_accepted() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let result = PredictionMarketContractClient::new(&env, &cid).try_initialize(
        &admin, &treasury, &oracle, &token,
        &200u32, &100u32, &1_000i128, &100i128, &256u32, &500i128,
    );
    assert!(result.is_ok());
}

#[test]
fn test_zero_dispute_bond_rejected() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let result = PredictionMarketContractClient::new(&env, &cid).try_initialize(
        &admin, &treasury, &oracle, &token,
        &200u32, &100u32, &1_000i128, &100i128, &2u32, &0i128,
    );
    assert_eq!(result, Err(Ok(PredictionMarketError::InvalidDisputeBond)));
}

// ---------------------------------------------------------------------------
// No partial writes on failure
// ---------------------------------------------------------------------------

#[test]
fn test_no_partial_writes_on_failure() {
    let (env, cid, admin, treasury, oracle, token) = setup();
    let client = PredictionMarketContractClient::new(&env, &cid);

    // Trigger FeesTooHigh – nothing should be written
    let _ = client.try_initialize(
        &admin, &treasury, &oracle, &token,
        &9_000u32, &2_000u32,
        &1_000i128, &100i128, &2u32, &500i128,
    );

    assert!(client.get_config().is_none());
    assert_eq!(client.get_next_market_id(), 0u64);
    assert!(!client.is_paused());
}
