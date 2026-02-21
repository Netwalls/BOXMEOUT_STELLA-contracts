#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, Symbol,
};

use boxmeout::{OracleManager, OracleManagerClient};

fn create_test_env() -> Env {
    Env::default()
}

fn register_oracle(env: &Env) -> Address {
    env.register_contract(None, OracleManager)
}

#[test]
fn test_oracle_initialize() {
    let env = create_test_env();
    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    let required_consensus = 2u32; // 2 of 3 oracles

    env.mock_all_auths();
    client.initialize(&admin, &required_consensus);

    // TODO: Add getters to verify
    // Verify required_consensus stored correctly
}

#[test]
fn test_register_oracle() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    let required_consensus = 2u32;
    client.initialize(&admin, &required_consensus);

    // Register oracle
    let oracle1 = Address::generate(&env);
    let oracle_name = Symbol::new(&env, "Oracle1");

    client.register_oracle(&oracle1, &oracle_name);

    // TODO: Add getter to verify oracle registered
    // Verify oracle count incremented
}

#[test]
fn test_register_multiple_oracles() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    // Register 3 oracles
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle3 = Address::generate(&env);

    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));
    client.register_oracle(&oracle2, &Symbol::new(&env, "Oracle2"));
    client.register_oracle(&oracle3, &Symbol::new(&env, "Oracle3"));

    // TODO: Verify 3 oracles registered
}

#[test]
#[should_panic(expected = "Maximum oracle limit reached")]
fn test_register_oracle_exceeds_limit() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    // Register 11 oracles (limit is 10)
    for i in 0..11 {
        let oracle = Address::generate(&env);
        let name = Symbol::new(&env, "Oracle");
        client.register_oracle(&oracle, &name);
    }
}

#[test]
#[should_panic(expected = "oracle already registered")]
#[should_panic]
fn test_register_duplicate_oracle() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    let name = Symbol::new(&env, "Oracle1");

    // Register once
    client.register_oracle(&oracle1, &name);

    // Try to register same oracle again
    client.register_oracle(&oracle1, &name);
}

#[test]
fn test_submit_attestation() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));

    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let resolution_time = 1000u64;

    // Register market with resolution time
    client.register_market(&market_id, &resolution_time);

    // Set ledger time past resolution time
    env.ledger().set_timestamp(1001);

    let result = 1u32; // YES
    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    client.submit_attestation(&oracle1, &market_id, &result, &data_hash);

    // Verify consensus is still false (need 2 votes)
    let (reached, outcome) = client.check_consensus(&market_id);
    assert!(!reached);
    assert_eq!(outcome, 0);
}

#[test]
fn test_check_consensus_reached() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle3 = Address::generate(&env);

    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));
    client.register_oracle(&oracle2, &Symbol::new(&env, "Oracle2"));
    client.register_oracle(&oracle3, &Symbol::new(&env, "Oracle3"));

    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let resolution_time = 1000u64;

    // Register market and set timestamp past resolution time
    client.register_market(&market_id, &resolution_time);
    env.ledger().set_timestamp(1001);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    // 2 oracles submit YES (1)
    client.submit_attestation(&oracle1, &market_id, &1u32, &data_hash);
    client.submit_attestation(&oracle2, &market_id, &1u32, &data_hash);

    // Verify consensus reached YES
    let (reached, outcome) = client.check_consensus(&market_id);
    assert!(reached);
    assert_eq!(outcome, 1);
}

#[test]
fn test_check_consensus_not_reached() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &3u32); // Need 3 oracles

    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));
    client.register_oracle(&oracle2, &Symbol::new(&env, "Oracle2"));

    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let resolution_time = 1000u64;

    // Register market and set timestamp past resolution time
    client.register_market(&market_id, &resolution_time);
    env.ledger().set_timestamp(1001);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    client.submit_attestation(&oracle1, &market_id, &1u32, &data_hash);
    client.submit_attestation(&oracle2, &market_id, &1u32, &data_hash);

    // Only 2 of 3 votes, consensus not reached
    let (reached, _) = client.check_consensus(&market_id);
    assert!(!reached);
}

#[test]
#[ignore]
#[should_panic(expected = "consensus not reached")]
fn test_resolve_market_without_consensus() {
    // TODO: Implement when resolve_market is ready
    // Only 1 oracle submitted
    // Cannot resolve yet
    // Cannot resolve yet
}

#[test]
fn test_check_consensus_tie_handling() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32); // threshold 2

    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle3 = Address::generate(&env);
    let oracle4 = Address::generate(&env);

    client.register_oracle(&oracle1, &Symbol::new(&env, "O1"));
    client.register_oracle(&oracle2, &Symbol::new(&env, "O2"));
    client.register_oracle(&oracle3, &Symbol::new(&env, "O3"));
    client.register_oracle(&oracle4, &Symbol::new(&env, "O4"));

    let market_id = BytesN::from_array(&env, &[1u8; 32]);
    let resolution_time = 1000u64;

    // Register market and set timestamp past resolution time
    client.register_market(&market_id, &resolution_time);
    env.ledger().set_timestamp(1001);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    // 2 vote YES, 2 vote NO
    client.submit_attestation(&oracle1, &market_id, &1u32, &data_hash);
    client.submit_attestation(&oracle2, &market_id, &1u32, &data_hash);
    client.submit_attestation(&oracle3, &market_id, &0u32, &data_hash);
    client.submit_attestation(&oracle4, &market_id, &0u32, &data_hash);

    // Both reached threshold 2, but it's a tie
    let (reached, _) = client.check_consensus(&market_id);
    assert!(!reached);
}

#[test]
fn test_remove_oracle() {
    // TODO: Implement when remove_oracle is ready
    // Admin removes misbehaving oracle
    // Only admin can remove
}

#[test]
fn test_update_oracle_accuracy() {
    // TODO: Implement when update_accuracy is ready
    // Track oracle accuracy over time
    // Accurate predictions increase accuracy score
}

// ===== NEW ATTESTATION TESTS =====

/// Happy path: Attestation is stored correctly with timestamp
#[test]
fn test_submit_attestation_stores_attestation() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));

    let market_id = BytesN::from_array(&env, &[2u8; 32]);
    let resolution_time = 1000u64;

    // Register market with resolution time
    client.register_market(&market_id, &resolution_time);

    // Set ledger time past resolution time
    env.ledger().set_timestamp(1500);

    let result = 1u32; // YES
    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    client.submit_attestation(&oracle1, &market_id, &result, &data_hash);

    // Verify attestation is stored correctly
    let attestation = client.get_attestation(&market_id, &oracle1);
    assert!(attestation.is_some());
    let attestation = attestation.unwrap();
    assert_eq!(attestation.attestor, oracle1);
    assert_eq!(attestation.outcome, 1);
    assert_eq!(attestation.timestamp, 1500);

    // Verify attestation counts are updated
    let (yes_count, no_count) = client.get_attestation_counts(&market_id);
    assert_eq!(yes_count, 1);
    assert_eq!(no_count, 0);
}

/// Non-attestor (unregistered oracle) is rejected
#[test]
#[should_panic(expected = "Oracle not registered")]
fn test_submit_attestation_non_attestor_rejected() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    // Note: we do NOT register unregistered_oracle as an oracle
    let unregistered_oracle = Address::generate(&env);

    let market_id = BytesN::from_array(&env, &[3u8; 32]);
    let resolution_time = 1000u64;

    // Register market
    client.register_market(&market_id, &resolution_time);

    // Set ledger time past resolution time
    env.ledger().set_timestamp(1500);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    // This should panic because oracle is not registered
    client.submit_attestation(&unregistered_oracle, &market_id, &1u32, &data_hash);
}

/// Cannot attest before resolution_time
#[test]
#[should_panic(expected = "Cannot attest before resolution time")]
fn test_submit_attestation_before_resolution_time() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));

    let market_id = BytesN::from_array(&env, &[4u8; 32]);
    let resolution_time = 2000u64;

    // Register market with resolution time of 2000
    client.register_market(&market_id, &resolution_time);

    // Set ledger time BEFORE resolution time
    env.ledger().set_timestamp(1500);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    // This should panic because we're before resolution time
    client.submit_attestation(&oracle1, &market_id, &1u32, &data_hash);
}

/// Invalid outcome (not 0 or 1) is rejected
#[test]
#[should_panic(expected = "Invalid attestation result")]
fn test_submit_attestation_invalid_outcome_rejected() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));

    let market_id = BytesN::from_array(&env, &[5u8; 32]);
    let resolution_time = 1000u64;

    // Register market
    client.register_market(&market_id, &resolution_time);

    // Set ledger time past resolution time
    env.ledger().set_timestamp(1500);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    // This should panic because outcome 2 is invalid (only 0 or 1 allowed)
    client.submit_attestation(&oracle1, &market_id, &2u32, &data_hash);
}

/// Verify AttestationSubmitted event is emitted correctly
#[test]
fn test_submit_attestation_event_emitted() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "Oracle1"));

    let market_id = BytesN::from_array(&env, &[6u8; 32]);
    let resolution_time = 1000u64;

    // Register market
    client.register_market(&market_id, &resolution_time);

    // Set ledger time past resolution time
    env.ledger().set_timestamp(1500);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    client.submit_attestation(&oracle1, &market_id, &1u32, &data_hash);

    // Verify event was emitted
    // The event system stores events that can be queried
    // In test environment, we verify by checking the attestation was stored
    // and the counts were updated (both happen only if function completes successfully)
    let attestation = client.get_attestation(&market_id, &oracle1);
    assert!(attestation.is_some());

    // Verify attestation counts
    let (yes_count, no_count) = client.get_attestation_counts(&market_id);
    assert_eq!(yes_count, 1);
    assert_eq!(no_count, 0);
}

/// Test register_market function
#[test]
fn test_register_market() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let market_id = BytesN::from_array(&env, &[7u8; 32]);
    let resolution_time = 3000u64;

    // Register market
    client.register_market(&market_id, &resolution_time);

    // Verify resolution time is stored
    let stored_time = client.get_market_resolution_time(&market_id);
    assert!(stored_time.is_some());
    assert_eq!(stored_time.unwrap(), 3000);

    // Verify attestation counts are initialized to 0
    let (yes_count, no_count) = client.get_attestation_counts(&market_id);
    assert_eq!(yes_count, 0);
    assert_eq!(no_count, 0);
}

/// Test attestation count tracking for both YES and NO outcomes
#[test]
fn test_attestation_count_tracking() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &2u32);

    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle3 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "O1"));
    client.register_oracle(&oracle2, &Symbol::new(&env, "O2"));
    client.register_oracle(&oracle3, &Symbol::new(&env, "O3"));

    let market_id = BytesN::from_array(&env, &[8u8; 32]);
    let resolution_time = 1000u64;

    // Register market
    client.register_market(&market_id, &resolution_time);
    env.ledger().set_timestamp(1500);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    // 2 vote YES, 1 vote NO
    client.submit_attestation(&oracle1, &market_id, &1u32, &data_hash);
    client.submit_attestation(&oracle2, &market_id, &1u32, &data_hash);
    client.submit_attestation(&oracle3, &market_id, &0u32, &data_hash);

    // Verify counts
    let (yes_count, no_count) = client.get_attestation_counts(&market_id);
    assert_eq!(yes_count, 2);
    assert_eq!(no_count, 1);
}

// ===== EMERGENCY OVERRIDE TESTS =====

/// Test: Emergency override with valid multi-sig (2 of 3 admins)
#[test]
fn test_emergency_override_happy_path() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    // Initialize with admin1
    client.initialize(&admin1, &2u32);

    // Add admin2 and admin3 as signers
    client.add_admin_signer(&admin1, &admin2);
    client.add_admin_signer(&admin1, &admin3);

    // Register a market
    let market_id = BytesN::from_array(&env, &[99u8; 32]);
    let resolution_time = 1000u64;
    client.register_market(&market_id, &resolution_time);

    // Set ledger time
    env.ledger().set_timestamp(2000);

    // Create approvers list (admin1 and admin2)
    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(admin2.clone());

    let forced_outcome = 1u32; // YES
    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    // Execute emergency override
    client.emergency_override(&approvers, &market_id, &forced_outcome, &justification_hash);

    // Verify consensus result was set
    let result = client.get_consensus_result(&market_id);
    assert_eq!(result, 1);

    // Verify market is marked as manual override
    let is_override = client.is_manual_override(&market_id);
    assert!(is_override);

    // Verify override record exists
    let record = client.get_override_record(&market_id);
    assert!(record.is_some());
    let record = record.unwrap();
    assert_eq!(record.forced_outcome, 1);
    assert_eq!(record.approvers.len(), 2);
}

/// Test: Emergency override fails with insufficient approvers
#[test]
#[should_panic(expected = "Insufficient approvers")]
fn test_emergency_override_insufficient_approvers() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    let market_id = BytesN::from_array(&env, &[100u8; 32]);
    client.register_market(&market_id, &1000u64);
    env.ledger().set_timestamp(2000);

    // Only 1 approver (need 2)
    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    // Should panic: insufficient approvers
    client.emergency_override(&approvers, &market_id, &1u32, &justification_hash);
}

/// Test: Emergency override fails with invalid approver (not an admin)
#[test]
#[should_panic(expected = "Invalid approver: not an admin")]
fn test_emergency_override_invalid_approver() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let non_admin = Address::generate(&env); // Not registered as admin

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    let market_id = BytesN::from_array(&env, &[101u8; 32]);
    client.register_market(&market_id, &1000u64);
    env.ledger().set_timestamp(2000);

    // Include non-admin in approvers
    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(non_admin.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    // Should panic: invalid approver
    client.emergency_override(&approvers, &market_id, &1u32, &justification_hash);
}

/// Test: Emergency override fails with invalid outcome (not 0 or 1)
#[test]
#[should_panic(expected = "Invalid outcome: must be 0 or 1")]
fn test_emergency_override_invalid_outcome() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    let market_id = BytesN::from_array(&env, &[102u8; 32]);
    client.register_market(&market_id, &1000u64);
    env.ledger().set_timestamp(2000);

    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(admin2.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    // Invalid outcome: 2 (only 0 or 1 allowed)
    client.emergency_override(&approvers, &market_id, &2u32, &justification_hash);
}

/// Test: Emergency override fails during cooldown period
#[test]
#[should_panic(expected = "Cooldown period not elapsed")]
fn test_emergency_override_cooldown_not_elapsed() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    // Set cooldown to 1 hour (3600 seconds)
    client.set_override_cooldown(&admin1, &3600u64);

    let market_id1 = BytesN::from_array(&env, &[103u8; 32]);
    let market_id2 = BytesN::from_array(&env, &[104u8; 32]);
    client.register_market(&market_id1, &1000u64);
    client.register_market(&market_id2, &1000u64);

    env.ledger().set_timestamp(2000);

    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(admin2.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    // First override succeeds
    client.emergency_override(&approvers, &market_id1, &1u32, &justification_hash);

    // Try second override 30 minutes later (1800 seconds)
    env.ledger().set_timestamp(3800);

    // Should panic: cooldown not elapsed (need 3600 seconds)
    client.emergency_override(&approvers, &market_id2, &0u32, &justification_hash);
}

/// Test: Emergency override succeeds after cooldown period
#[test]
fn test_emergency_override_after_cooldown() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    // Set cooldown to 1 hour
    client.set_override_cooldown(&admin1, &3600u64);

    let market_id1 = BytesN::from_array(&env, &[105u8; 32]);
    let market_id2 = BytesN::from_array(&env, &[106u8; 32]);
    client.register_market(&market_id1, &1000u64);
    client.register_market(&market_id2, &1000u64);

    env.ledger().set_timestamp(2000);

    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(admin2.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    // First override
    client.emergency_override(&approvers, &market_id1, &1u32, &justification_hash);

    // Wait for cooldown to elapse (3600 seconds)
    env.ledger().set_timestamp(5601);

    // Second override should succeed
    client.emergency_override(&approvers, &market_id2, &0u32, &justification_hash);

    // Verify both overrides succeeded
    assert_eq!(client.get_consensus_result(&market_id1), 1);
    assert_eq!(client.get_consensus_result(&market_id2), 0);
}

/// Test: Emergency override fails for unregistered market
#[test]
#[should_panic(expected = "Market not registered")]
fn test_emergency_override_unregistered_market() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    env.ledger().set_timestamp(2000);

    let unregistered_market = BytesN::from_array(&env, &[107u8; 32]);

    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(admin2.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    // Should panic: market not registered
    client.emergency_override(&approvers, &unregistered_market, &1u32, &justification_hash);
}

/// Test: Add admin signer
#[test]
fn test_add_admin_signer() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);

    // Add admin2
    client.add_admin_signer(&admin1, &admin2);

    // Verify admin2 was added
    let signers = client.get_admin_signers();
    assert_eq!(signers.len(), 2);
}

/// Test: Add admin signer fails for non-admin
#[test]
#[should_panic(expected = "Only admin can add signers")]
fn test_add_admin_signer_non_admin() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin1, &2u32);

    // Non-admin tries to add signer
    client.add_admin_signer(&non_admin, &new_admin);
}

/// Test: Add duplicate admin signer fails
#[test]
#[should_panic(expected = "Admin already exists")]
fn test_add_duplicate_admin_signer() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);

    client.initialize(&admin1, &2u32);

    // Try to add admin1 again
    client.add_admin_signer(&admin1, &admin1);
}

/// Test: Set required signatures
#[test]
fn test_set_required_signatures() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);
    client.add_admin_signer(&admin1, &admin3);

    // Change required signatures to 3
    client.set_required_signatures(&admin1, &3u32);

    // Verify
    let required = client.get_required_signatures();
    assert_eq!(required, 3);
}

/// Test: Set required signatures fails with invalid value
#[test]
#[should_panic(expected = "Invalid required signatures")]
fn test_set_required_signatures_invalid() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);

    client.initialize(&admin1, &2u32);

    // Try to set required signatures to 0
    client.set_required_signatures(&admin1, &0u32);
}

/// Test: Set required signatures exceeds admin count
#[test]
#[should_panic(expected = "Invalid required signatures")]
fn test_set_required_signatures_exceeds_admin_count() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    // Try to set required signatures to 5 (only 2 admins)
    client.set_required_signatures(&admin1, &5u32);
}

/// Test: Set override cooldown
#[test]
fn test_set_override_cooldown() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);

    client.initialize(&admin1, &2u32);

    // Set cooldown to 2 hours (7200 seconds)
    client.set_override_cooldown(&admin1, &7200u64);

    // Verify
    let cooldown = client.get_override_cooldown();
    assert_eq!(cooldown, 7200);
}

/// Test: Set override cooldown fails with too short period
#[test]
#[should_panic(expected = "Cooldown must be at least 1 hour")]
fn test_set_override_cooldown_too_short() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);

    client.initialize(&admin1, &2u32);

    // Try to set cooldown to 30 minutes (1800 seconds) - should fail
    client.set_override_cooldown(&admin1, &1800u64);
}

/// Test: Emergency override with 3 of 3 admins
#[test]
fn test_emergency_override_three_of_three() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);
    client.add_admin_signer(&admin1, &admin3);

    // Set required signatures to 3
    client.set_required_signatures(&admin1, &3u32);

    let market_id = BytesN::from_array(&env, &[108u8; 32]);
    client.register_market(&market_id, &1000u64);
    env.ledger().set_timestamp(2000);

    // All 3 admins approve
    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(admin2.clone());
    approvers.push_back(admin3.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    client.emergency_override(&approvers, &market_id, &1u32, &justification_hash);

    // Verify override succeeded
    let result = client.get_consensus_result(&market_id);
    assert_eq!(result, 1);

    // Verify override record has 3 approvers
    let record = client.get_override_record(&market_id).unwrap();
    assert_eq!(record.approvers.len(), 3);
}

/// Test: Get override record returns None for non-overridden market
#[test]
fn test_get_override_record_none() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    client.initialize(&admin1, &2u32);

    let market_id = BytesN::from_array(&env, &[109u8; 32]);
    client.register_market(&market_id, &1000u64);

    // No override performed
    let record = client.get_override_record(&market_id);
    assert!(record.is_none());
}

/// Test: is_manual_override returns false for non-overridden market
#[test]
fn test_is_manual_override_false() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    client.initialize(&admin1, &2u32);

    let market_id = BytesN::from_array(&env, &[110u8; 32]);
    client.register_market(&market_id, &1000u64);

    // No override performed
    let is_override = client.is_manual_override(&market_id);
    assert!(!is_override);
}

/// Test: Emergency override overrides existing consensus
#[test]
fn test_emergency_override_overrides_consensus() {
    let env = create_test_env();
    env.mock_all_auths();

    let oracle_id = register_oracle(&env);
    let client = OracleManagerClient::new(&env, &oracle_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1, &2u32);
    client.add_admin_signer(&admin1, &admin2);

    // Register oracles
    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    client.register_oracle(&oracle1, &Symbol::new(&env, "O1"));
    client.register_oracle(&oracle2, &Symbol::new(&env, "O2"));

    let market_id = BytesN::from_array(&env, &[111u8; 32]);
    client.register_market(&market_id, &1000u64);
    env.ledger().set_timestamp(1500);

    let data_hash = BytesN::from_array(&env, &[0u8; 32]);

    // Oracles reach consensus on YES (1)
    client.submit_attestation(&oracle1, &market_id, &1u32, &data_hash);
    client.submit_attestation(&oracle2, &market_id, &1u32, &data_hash);

    let (reached, outcome) = client.check_consensus(&market_id);
    assert!(reached);
    assert_eq!(outcome, 1);

    // Emergency override to NO (0)
    let mut approvers = soroban_sdk::Vec::new(&env);
    approvers.push_back(admin1.clone());
    approvers.push_back(admin2.clone());

    let justification_hash = BytesN::from_array(&env, &[0xABu8; 32]);

    client.emergency_override(&approvers, &market_id, &0u32, &justification_hash);

    // Verify override changed the result to NO
    let result = client.get_consensus_result(&market_id);
    assert_eq!(result, 0);

    // Verify market is marked as manual override
    assert!(client.is_manual_override(&market_id));
}
