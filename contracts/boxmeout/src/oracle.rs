// contract/src/oracle.rs - Oracle & Market Resolution Contract Implementation
// Handles multi-source oracle consensus for market resolution

use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Symbol, Vec};

// Storage keys
const ADMIN_KEY: &str = "admin";
const REQUIRED_CONSENSUS_KEY: &str = "required_consensus";
const ORACLE_COUNT_KEY: &str = "oracle_count";
const MARKET_RES_TIME_KEY: &str = "mkt_res_time"; // Market resolution time storage
const ATTEST_COUNT_YES_KEY: &str = "attest_yes"; // Attestation count for YES outcome
const ATTEST_COUNT_NO_KEY: &str = "attest_no"; // Attestation count for NO outcome
const ADMIN_SIGNERS_KEY: &str = "admin_signers"; // Multi-sig admin addresses
const REQUIRED_SIGNATURES_KEY: &str = "required_sigs"; // Required signatures for multi-sig
const LAST_OVERRIDE_TIME_KEY: &str = "last_override"; // Timestamp of last emergency override
const OVERRIDE_COOLDOWN_KEY: &str = "override_cooldown"; // Cooldown period in seconds (default 86400 = 24h)

/// Attestation record for market resolution
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    pub attestor: Address,
    pub outcome: u32,
    pub timestamp: u64,
}

/// Emergency override approval record
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverrideApproval {
    pub admin: Address,
    pub timestamp: u64,
}

/// Emergency override record for audit trail
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyOverrideRecord {
    pub market_id: BytesN<32>,
    pub forced_outcome: u32,
    pub justification_hash: BytesN<32>,
    pub approvers: Vec<Address>,
    pub timestamp: u64,
}

/// ORACLE MANAGER - Manages oracle consensus
#[contract]
pub struct OracleManager;

#[contractimpl]
impl OracleManager {
    /// Initialize oracle system with validator set and multi-sig admins
    pub fn initialize(env: Env, admin: Address, required_consensus: u32) {
        // Verify admin signature
        admin.require_auth();

        // Store admin
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, ADMIN_KEY), &admin);

        // Store required consensus threshold
        env.storage().persistent().set(
            &Symbol::new(&env, REQUIRED_CONSENSUS_KEY),
            &required_consensus,
        );

        // Initialize oracle counter
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, ORACLE_COUNT_KEY), &0u32);

        // Initialize multi-sig with single admin (can be updated later)
        let mut admin_signers = Vec::new(&env);
        admin_signers.push_back(admin.clone());
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, ADMIN_SIGNERS_KEY), &admin_signers);

        // Default: require 2 of 3 signatures for emergency override
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, REQUIRED_SIGNATURES_KEY), &2u32);

        // Default cooldown: 24 hours (86400 seconds)
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, OVERRIDE_COOLDOWN_KEY), &86400u64);

        // Initialize last override time to 0
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, LAST_OVERRIDE_TIME_KEY), &0u64);

        // Emit initialization event
        env.events().publish(
            (Symbol::new(&env, "oracle_initialized"),),
            (admin, required_consensus),
        );
    }

    /// Register a new oracle node
    pub fn register_oracle(env: Env, oracle: Address, oracle_name: Symbol) {
        // Require admin authentication
        let admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, ADMIN_KEY))
            .unwrap();
        admin.require_auth();

        // Get current oracle count
        let oracle_count: u32 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, ORACLE_COUNT_KEY))
            .unwrap_or(0);

        // Validate total_oracles < max_oracles (max 10 oracles)
        if oracle_count >= 10 {
            panic!("Maximum oracle limit reached");
        }

        // Create storage key for this oracle using the oracle address
        let oracle_key = (Symbol::new(&env, "oracle"), oracle.clone());

        // Check if oracle already registered
        let is_registered: bool = env.storage().persistent().has(&oracle_key);

        if is_registered {
            panic!("Oracle already registered");
        }

        // Store oracle metadata
        env.storage().persistent().set(&oracle_key, &true);

        // Store oracle name
        let oracle_name_key = (Symbol::new(&env, "oracle_name"), oracle.clone());
        env.storage()
            .persistent()
            .set(&oracle_name_key, &oracle_name);

        // Initialize oracle's accuracy score at 100%
        let accuracy_key = (Symbol::new(&env, "oracle_accuracy"), oracle.clone());
        env.storage().persistent().set(&accuracy_key, &100u32);

        // Store registration timestamp
        let timestamp_key = (Symbol::new(&env, "oracle_timestamp"), oracle.clone());
        env.storage()
            .persistent()
            .set(&timestamp_key, &env.ledger().timestamp());

        // Increment oracle counter
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, ORACLE_COUNT_KEY), &(oracle_count + 1));

        // Emit OracleRegistered event
        env.events().publish(
            (Symbol::new(&env, "oracle_registered"),),
            (oracle, oracle_name, env.ledger().timestamp()),
        );
    }

    /// Deregister an oracle node
    ///
    /// TODO: Deregister Oracle
    /// - Require admin authentication
    /// - Validate oracle is registered
    /// - Remove oracle from active_oracles list
    /// - Mark as inactive (don't delete, keep for history)
    /// - Prevent oracle from submitting new attestations
    /// - Don't affect existing attestations
    /// - Emit OracleDeregistered(oracle_address, timestamp)
    pub fn deregister_oracle(env: Env, oracle: Address) {
        todo!("See deregister oracle TODO above")
    }

    /// Register a market with its resolution time for attestation validation
    /// Must be called before oracles can submit attestations for this market.
    pub fn register_market(env: Env, market_id: BytesN<32>, resolution_time: u64) {
        // Require admin authentication
        let admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, ADMIN_KEY))
            .expect("Oracle not initialized");
        admin.require_auth();

        // Store market resolution time
        let market_key = (Symbol::new(&env, MARKET_RES_TIME_KEY), market_id.clone());
        env.storage()
            .persistent()
            .set(&market_key, &resolution_time);

        // Initialize attestation counts for this market
        let yes_count_key = (Symbol::new(&env, ATTEST_COUNT_YES_KEY), market_id.clone());
        let no_count_key = (Symbol::new(&env, ATTEST_COUNT_NO_KEY), market_id.clone());
        env.storage().persistent().set(&yes_count_key, &0u32);
        env.storage().persistent().set(&no_count_key, &0u32);

        // Emit market registered event
        env.events().publish(
            (Symbol::new(&env, "market_registered"),),
            (market_id, resolution_time),
        );
    }

    /// Get market resolution time (helper function)
    pub fn get_market_resolution_time(env: Env, market_id: BytesN<32>) -> Option<u64> {
        let market_key = (Symbol::new(&env, MARKET_RES_TIME_KEY), market_id);
        env.storage().persistent().get(&market_key)
    }

    /// Get attestation counts for a market
    pub fn get_attestation_counts(env: Env, market_id: BytesN<32>) -> (u32, u32) {
        let yes_count_key = (Symbol::new(&env, ATTEST_COUNT_YES_KEY), market_id.clone());
        let no_count_key = (Symbol::new(&env, ATTEST_COUNT_NO_KEY), market_id);

        let yes_count: u32 = env.storage().persistent().get(&yes_count_key).unwrap_or(0);
        let no_count: u32 = env.storage().persistent().get(&no_count_key).unwrap_or(0);

        (yes_count, no_count)
    }

    /// Get attestation record for an oracle on a market
    pub fn get_attestation(
        env: Env,
        market_id: BytesN<32>,
        oracle: Address,
    ) -> Option<Attestation> {
        let attestation_key = (Symbol::new(&env, "attestation"), market_id, oracle);
        env.storage().persistent().get(&attestation_key)
    }

    /// Submit oracle attestation for market result
    ///
    /// Validates:
    /// - Caller is a trusted attestor (registered oracle)
    /// - Market is past resolution_time
    /// - Outcome is valid (0=NO, 1=YES)
    /// - Oracle hasn't already attested
    pub fn submit_attestation(
        env: Env,
        oracle: Address,
        market_id: BytesN<32>,
        attestation_result: u32,
        _data_hash: BytesN<32>,
    ) {
        // 1. Require oracle authentication
        oracle.require_auth();

        // 2. Validate oracle is registered (trusted attestor)
        let oracle_key = (Symbol::new(&env, "oracle"), oracle.clone());
        let is_registered: bool = env.storage().persistent().get(&oracle_key).unwrap_or(false);
        if !is_registered {
            panic!("Oracle not registered");
        }

        // 3. Validate market is registered and past resolution_time
        let market_key = (Symbol::new(&env, MARKET_RES_TIME_KEY), market_id.clone());
        let resolution_time: u64 = env
            .storage()
            .persistent()
            .get(&market_key)
            .expect("Market not registered");

        let current_time = env.ledger().timestamp();
        if current_time < resolution_time {
            panic!("Cannot attest before resolution time");
        }

        // 4. Validate result is binary (0 or 1)
        if attestation_result > 1 {
            panic!("Invalid attestation result");
        }

        // 5. Check if oracle already attested
        let vote_key = (Symbol::new(&env, "vote"), market_id.clone(), oracle.clone());
        if env.storage().persistent().has(&vote_key) {
            panic!("Oracle already attested");
        }

        // 6. Store vote for consensus
        env.storage()
            .persistent()
            .set(&vote_key, &attestation_result);

        // 7. Store attestation with timestamp
        let attestation = Attestation {
            attestor: oracle.clone(),
            outcome: attestation_result,
            timestamp: current_time,
        };
        let attestation_key = (
            Symbol::new(&env, "attestation"),
            market_id.clone(),
            oracle.clone(),
        );
        env.storage()
            .persistent()
            .set(&attestation_key, &attestation);

        // 8. Track oracle in market's voter list
        let voters_key = (Symbol::new(&env, "voters"), market_id.clone());
        let mut voters: Vec<Address> = env
            .storage()
            .persistent()
            .get(&voters_key)
            .unwrap_or(Vec::new(&env));

        voters.push_back(oracle.clone());
        env.storage().persistent().set(&voters_key, &voters);

        // 9. Update attestation count per outcome
        if attestation_result == 1 {
            let yes_count_key = (Symbol::new(&env, ATTEST_COUNT_YES_KEY), market_id.clone());
            let current_count: u32 = env.storage().persistent().get(&yes_count_key).unwrap_or(0);
            env.storage()
                .persistent()
                .set(&yes_count_key, &(current_count + 1));
        } else {
            let no_count_key = (Symbol::new(&env, ATTEST_COUNT_NO_KEY), market_id.clone());
            let current_count: u32 = env.storage().persistent().get(&no_count_key).unwrap_or(0);
            env.storage()
                .persistent()
                .set(&no_count_key, &(current_count + 1));
        }

        // 10. Emit AttestationSubmitted(market_id, attestor, outcome)
        env.events().publish(
            (Symbol::new(&env, "AttestationSubmitted"),),
            (market_id, oracle, attestation_result),
        );
    }

    /// Check if consensus has been reached for market
    pub fn check_consensus(env: Env, market_id: BytesN<32>) -> (bool, u32) {
        // 1. Query attestations for market_id
        let voters_key = (Symbol::new(&env, "voters"), market_id.clone());
        let voters: Vec<Address> = env
            .storage()
            .persistent()
            .get(&voters_key)
            .unwrap_or(Vec::new(&env));

        // 2. Get required threshold
        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, REQUIRED_CONSENSUS_KEY))
            .unwrap_or(0);

        if voters.len() < threshold {
            return (false, 0);
        }

        // 3. Count votes for each outcome
        let mut yes_votes = 0;
        let mut no_votes = 0;

        for oracle in voters.iter() {
            let vote_key = (Symbol::new(&env, "vote"), market_id.clone(), oracle);
            let vote: u32 = env.storage().persistent().get(&vote_key).unwrap_or(0);
            if vote == 1 {
                yes_votes += 1;
            } else {
                no_votes += 1;
            }
        }

        // 4. Compare counts against threshold
        // Winner is the one that reached the threshold first
        // If both reach threshold (possible if threshold is low), we favor the one with more votes
        // If tied and both >= threshold, return false (no clear winner yet)
        if yes_votes >= threshold && yes_votes > no_votes {
            (true, 1)
        } else if no_votes >= threshold && no_votes > yes_votes {
            (true, 0)
        } else if yes_votes >= threshold && no_votes >= threshold && yes_votes == no_votes {
            // Tie scenario appropriately handled: no consensus if tied but threshold met
            (false, 0)
        } else {
            (false, 0)
        }
    }

    /// Get the consensus result for a market
    pub fn get_consensus_result(env: Env, market_id: BytesN<32>) -> u32 {
        let result_key = (Symbol::new(&env, "consensus_result"), market_id.clone());
        env.storage()
            .persistent()
            .get(&result_key)
            .expect("Consensus result not found")
    }

    /// Finalize market resolution after time delay
    ///
    /// TODO: Finalize Resolution
    /// - Validate market_id exists
    /// - Validate consensus already reached
    /// - Validate time_delay_before_finality has passed
    /// - Validate no active disputes/challenges
    /// - Get consensus_result
    /// - Call market contract's resolve_market() function
    /// - Pass winning_outcome to market
    /// - Confirm resolution recorded
    /// - Emit ResolutionFinalized(market_id, outcome, timestamp)
    pub fn finalize_resolution(env: Env, market_id: BytesN<32>) {
        todo!("See finalize resolution TODO above")
    }

    /// Challenge an attestation (dispute oracle honesty)
    ///
    /// TODO: Challenge Attestation
    /// - Require challenger authentication (must be oracle or participant)
    /// - Validate market_id and oracle being challenged
    /// - Validate attestation exists
    /// - Create challenge record: { challenger, oracle_challenged, reason, timestamp }
    /// - Pause consensus finalization until challenge resolved
    /// - Emit AttestationChallenged(oracle, challenger, market_id, reason)
    /// - Require evidence/proof in challenge
    pub fn challenge_attestation(
        env: Env,
        challenger: Address,
        oracle: Address,
        market_id: BytesN<32>,
        challenge_reason: Symbol,
    ) {
        todo!("See challenge attestation TODO above")
    }

    /// Resolve a challenge and update oracle reputation
    ///
    /// TODO: Resolve Challenge
    /// - Require admin authentication
    /// - Query challenge record
    /// - Review evidence submitted
    /// - Determine if challenge is valid (oracle was dishonest)
    /// - If valid:
    ///   - Reduce oracle's reputation/accuracy score
    ///   - If score drops below threshold: deregister oracle
    ///   - Potentially slash oracle's stake (if implemented)
    /// - If invalid:
    ///   - Increase oracle's reputation
    ///   - Penalize false challenger
    /// - Emit ChallengeResolved(oracle, challenger, is_valid, new_reputation)
    pub fn resolve_challenge(
        env: Env,
        oracle: Address,
        market_id: BytesN<32>,
        challenge_valid: bool,
    ) {
        todo!("See resolve challenge TODO above")
    }

    /// Get all attestations for a market
    ///
    /// TODO: Get Attestations
    /// - Query attestations map by market_id
    /// - Return all oracles' attestations for this market
    /// - Include: oracle_address, result, data_hash, timestamp
    /// - Include: consensus status and vote counts
    pub fn get_attestations(env: Env, market_id: BytesN<32>) -> Vec<Symbol> {
        todo!("See get attestations TODO above")
    }

    /// Get oracle info and reputation
    ///
    /// TODO: Get Oracle Info
    /// - Query oracle_registry by oracle_address
    /// - Return: name, reputation_score, attestations_count, accuracy_pct
    /// - Include: joined_timestamp, status (active/inactive)
    /// - Include: challenges_received, challenges_won
    pub fn get_oracle_info(env: Env, oracle: Address) -> Symbol {
        todo!("See get oracle info TODO above")
    }

    /// Get all active oracles
    ///
    /// TODO: Get Active Oracles
    /// - Query oracle_registry for all oracles with status=active
    /// - Return list of oracle addresses
    /// - Include: reputation scores sorted by highest first
    /// - Include: availability status
    pub fn get_active_oracles(env: Env) -> Vec<Address> {
        todo!("See get active oracles TODO above")
    }

    /// Admin: Update oracle consensus threshold
    ///
    /// TODO: Set Consensus Threshold
    /// - Require admin authentication
    /// - Validate new_threshold > 0 and <= total_oracles
    /// - Validate reasonable (e.g., 2 of 3, 3 of 5, etc.)
    /// - Update required_consensus
    /// - Apply to future markets only
    /// - Emit ConsensusThresholdUpdated(new_threshold, old_threshold)
    pub fn set_consensus_threshold(env: Env, new_threshold: u32) {
        todo!("See set consensus threshold TODO above")
    }

    /// Get oracle consensus report
    ///
    /// TODO: Get Consensus Report
    /// - Compile oracle performance metrics
    /// - Return: total_markets_resolved, consensus_efficiency, dispute_rate
    /// - Include: by_oracle (each oracle's stats)
    /// - Include: time: average_time_to_consensus
    pub fn get_consensus_report(env: Env) -> Symbol {
        todo!("See get consensus report TODO above")
    }

    /// Add admin signer for multi-sig (only callable by existing admin)
    pub fn add_admin_signer(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();

        // Verify caller is an existing admin
        let admin_signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, ADMIN_SIGNERS_KEY))
            .expect("Oracle not initialized");

        let mut is_admin = false;
        for admin in admin_signers.iter() {
            if admin == caller {
                is_admin = true;
                break;
            }
        }

        if !is_admin {
            panic!("Only admin can add signers");
        }

        // Check if new_admin already exists
        for admin in admin_signers.iter() {
            if admin == new_admin {
                panic!("Admin already exists");
            }
        }

        // Add new admin
        let mut updated_signers = admin_signers;
        updated_signers.push_back(new_admin.clone());

        env.storage()
            .persistent()
            .set(&Symbol::new(&env, ADMIN_SIGNERS_KEY), &updated_signers);

        env.events().publish(
            (Symbol::new(&env, "admin_signer_added"),),
            (new_admin,),
        );
    }

    /// Set required signatures for emergency override (only callable by admin)
    pub fn set_required_signatures(env: Env, caller: Address, required_sigs: u32) {
        caller.require_auth();

        // Verify caller is admin
        let admin_signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, ADMIN_SIGNERS_KEY))
            .expect("Oracle not initialized");

        let mut is_admin = false;
        for admin in admin_signers.iter() {
            if admin == caller {
                is_admin = true;
                break;
            }
        }

        if !is_admin {
            panic!("Only admin can set required signatures");
        }

        // Validate required_sigs is reasonable
        if required_sigs == 0 || required_sigs > admin_signers.len() {
            panic!("Invalid required signatures");
        }

        env.storage()
            .persistent()
            .set(&Symbol::new(&env, REQUIRED_SIGNATURES_KEY), &required_sigs);

        env.events().publish(
            (Symbol::new(&env, "required_signatures_updated"),),
            (required_sigs,),
        );
    }

    /// Set override cooldown period (only callable by admin)
    pub fn set_override_cooldown(env: Env, caller: Address, cooldown_seconds: u64) {
        caller.require_auth();

        // Verify caller is admin
        let admin_signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, ADMIN_SIGNERS_KEY))
            .expect("Oracle not initialized");

        let mut is_admin = false;
        for admin in admin_signers.iter() {
            if admin == caller {
                is_admin = true;
                break;
            }
        }

        if !is_admin {
            panic!("Only admin can set cooldown");
        }

        // Minimum cooldown: 1 hour (3600 seconds)
        if cooldown_seconds < 3600 {
            panic!("Cooldown must be at least 1 hour");
        }

        env.storage()
            .persistent()
            .set(&Symbol::new(&env, OVERRIDE_COOLDOWN_KEY), &cooldown_seconds);

        env.events().publish(
            (Symbol::new(&env, "override_cooldown_updated"),),
            (cooldown_seconds,),
        );
    }

    /// Emergency: Override oracle consensus with multi-sig approval
    ///
    /// CRITICAL SAFETY MECHANISM - Requires multi-sig (at least 2 of 3 admins)
    /// Use only when oracle system is compromised or critical error detected
    ///
    /// Security Features:
    /// - Multi-sig requirement (configurable, default 2 of 3)
    /// - Cooldown period between overrides (default 24h)
    /// - Justification hash for audit trail
    /// - Complete override record stored permanently
    /// - EmergencyOverride event with all details
    ///
    /// Parameters:
    /// - approvers: Vec of admin addresses approving this override
    /// - market_id: Market to override
    /// - forced_outcome: Outcome to set (0=NO, 1=YES)
    /// - justification_hash: Hash of justification document (for transparency)
    pub fn emergency_override(
        env: Env,
        approvers: Vec<Address>,
        market_id: BytesN<32>,
        forced_outcome: u32,
        justification_hash: BytesN<32>,
    ) {
        // 1. Validate forced_outcome is binary (0 or 1)
        if forced_outcome > 1 {
            panic!("Invalid outcome: must be 0 or 1");
        }

        // 2. Get admin signers and required signatures
        let admin_signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, ADMIN_SIGNERS_KEY))
            .expect("Oracle not initialized");

        let required_sigs: u32 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, REQUIRED_SIGNATURES_KEY))
            .unwrap_or(2);

        // 3. Validate we have enough approvers
        if approvers.len() < required_sigs {
            panic!("Insufficient approvers");
        }

        // 4. Verify all approvers are valid admins and require their auth
        let mut valid_approver_count = 0u32;
        for approver in approvers.iter() {
            // Require authentication from each approver
            approver.require_auth();

            // Verify approver is in admin_signers list
            let mut is_valid_admin = false;
            for admin in admin_signers.iter() {
                if admin == approver {
                    is_valid_admin = true;
                    break;
                }
            }

            if !is_valid_admin {
                panic!("Invalid approver: not an admin");
            }

            valid_approver_count += 1;
        }

        // 5. Ensure no duplicate approvers (each admin can only approve once)
        if valid_approver_count != approvers.len() {
            panic!("Duplicate approvers detected");
        }

        // 6. Check cooldown period
        let last_override_time: u64 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, LAST_OVERRIDE_TIME_KEY))
            .unwrap_or(0);

        let cooldown_period: u64 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, OVERRIDE_COOLDOWN_KEY))
            .unwrap_or(86400);

        let current_time = env.ledger().timestamp();

        if last_override_time > 0 && (current_time - last_override_time) < cooldown_period {
            panic!("Cooldown period not elapsed");
        }

        // 7. Verify market exists
        let market_key = (Symbol::new(&env, MARKET_RES_TIME_KEY), market_id.clone());
        if !env.storage().persistent().has(&market_key) {
            panic!("Market not registered");
        }

        // 8. Store consensus result (override any existing consensus)
        let result_key = (Symbol::new(&env, "consensus_result"), market_id.clone());
        env.storage()
            .persistent()
            .set(&result_key, &forced_outcome);

        // 9. Mark market as manually overridden for audit purposes
        let override_flag_key = (Symbol::new(&env, "manual_override"), market_id.clone());
        env.storage().persistent().set(&override_flag_key, &true);

        // 10. Create and store complete override record
        let override_record = EmergencyOverrideRecord {
            market_id: market_id.clone(),
            forced_outcome,
            justification_hash: justification_hash.clone(),
            approvers: approvers.clone(),
            timestamp: current_time,
        };

        let override_record_key = (Symbol::new(&env, "override_record"), market_id.clone());
        env.storage()
            .persistent()
            .set(&override_record_key, &override_record);

        // 11. Update last override timestamp
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, LAST_OVERRIDE_TIME_KEY), &current_time);

        // 12. Emit EmergencyOverride event with all details
        env.events().publish(
            (Symbol::new(&env, "EmergencyOverride"),),
            (
                market_id,
                forced_outcome,
                justification_hash,
                approvers,
                current_time,
            ),
        );
    }

    /// Get emergency override record for a market (for audit purposes)
    pub fn get_override_record(env: Env, market_id: BytesN<32>) -> Option<EmergencyOverrideRecord> {
        let override_record_key = (Symbol::new(&env, "override_record"), market_id);
        env.storage().persistent().get(&override_record_key)
    }

    /// Check if market was manually overridden
    pub fn is_manual_override(env: Env, market_id: BytesN<32>) -> bool {
        let override_flag_key = (Symbol::new(&env, "manual_override"), market_id);
        env.storage()
            .persistent()
            .get(&override_flag_key)
            .unwrap_or(false)
    }

    /// Get admin signers list
    pub fn get_admin_signers(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&Symbol::new(&env, ADMIN_SIGNERS_KEY))
            .unwrap_or(Vec::new(&env))
    }

    /// Get required signatures for emergency override
    pub fn get_required_signatures(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&Symbol::new(&env, REQUIRED_SIGNATURES_KEY))
            .unwrap_or(2)
    }

    /// Get override cooldown period
    pub fn get_override_cooldown(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&Symbol::new(&env, OVERRIDE_COOLDOWN_KEY))
            .unwrap_or(86400)
    }

    /// Get last override timestamp
    pub fn get_last_override_time(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&Symbol::new(&env, LAST_OVERRIDE_TIME_KEY))
            .unwrap_or(0)
    }
}
