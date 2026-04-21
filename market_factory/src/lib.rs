/// ============================================================
/// BOXMEOUT — MarketFactory Contract
///
/// Responsibilities:
///   - Deploy and register new Market contracts
///   - Maintain the oracle whitelist
///   - Gate market creation (pause / unpause)
///   - Transfer admin rights
///
/// Contributors: implement every function marked todo!()
/// DO NOT change function signatures.
/// ============================================================

#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use boxmeout_shared::{
    errors::ContractError,
    types::{MarketConfig, MarketState, MarketStatus, FightDetails},
};

// ─── Storage Key Constants ────────────────────────────────────────────────────

/// u64 — monotonically increasing counter; also used as market_id
const MARKET_COUNT: &str = "MARKET_COUNT";
/// Map<u64, Address> — market_id → deployed Market contract address
const MARKET_MAP: &str = "MARKET_MAP";
/// Address — factory admin (should be a multisig account in production)
const ADMIN: &str = "ADMIN";
/// Vec<Address> — oracle addresses allowed to resolve markets
const ORACLE_WHITELIST: &str = "ORACLE_WHITELIST";
/// bool — when true, create_market() is rejected
const PAUSED: &str = "PAUSED";
/// MarketConfig — default config applied to new markets unless overridden
const DEFAULT_CONFIG: &str = "DEFAULT_CONFIG";

#[contract]
pub struct MarketFactory;

#[contractimpl]
impl MarketFactory {
    /// Initializes the factory.
    /// Stores admin, oracle whitelist, default config, and sets paused = false.
    /// Must be called exactly once immediately after deployment.
    /// Returns ContractError::AlreadyInitialized on a second call.
    pub fn initialize(
        env: Env,
        admin: Address,
        default_fee_bps: u32,
        oracles: Vec<Address>,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Creates a new prediction market for a boxing match.
    ///
    /// Steps:
    ///   1. Require factory is not paused
    ///   2. Require caller authorization
    ///   3. Validate fight details (scheduled_at in future, names non-empty)
    ///   4. Validate config (min_bet > 0, fee_bps <= 1000)
    ///   5. Deploy a new Market contract wasm
    ///   6. Call Market::initialize() on the new contract
    ///   7. Store market_id → contract_address in MARKET_MAP
    ///   8. Increment MARKET_COUNT
    ///   9. Emit MarketCreated event
    ///  10. Return the new market_id
    pub fn create_market(
        env: Env,
        caller: Address,
        fight: FightDetails,
        config: MarketConfig,
    ) -> Result<u64, ContractError> {
        todo!()
    }

    /// Returns the deployed Market contract address for a given market_id.
    /// Returns ContractError::MarketNotFound if the ID has not been registered.
    pub fn get_market_address(
        env: Env,
        market_id: u64,
    ) -> Result<Address, ContractError> {
        todo!()
    }

    /// Returns a paginated slice of (market_id, MarketStatus) pairs.
    /// offset: first index to return (0-based)
    /// limit:  maximum number of results (capped at 100 on-chain)
    /// Used by the backend indexer to discover all markets without scanning events.
    pub fn list_markets(
        env: Env,
        offset: u64,
        limit: u32,
    ) -> Vec<(u64, MarketStatus)> {
        todo!()
    }

    /// Returns the total number of markets ever created (includes cancelled ones).
    pub fn get_market_count(env: Env) -> u64 {
        todo!()
    }

    /// Adds an oracle address to the whitelist.
    /// Requires admin authorization.
    /// Idempotent — adding an already-present address is a no-op (no error).
    pub fn add_oracle(
        env: Env,
        admin: Address,
        oracle: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Removes an oracle address from the whitelist.
    /// Requires admin authorization.
    /// Returns ContractError::OracleNotWhitelisted if address is not present.
    pub fn remove_oracle(
        env: Env,
        admin: Address,
        oracle: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Returns the current oracle whitelist.
    pub fn get_oracles(env: Env) -> Vec<Address> {
        todo!()
    }

    /// Transfers factory admin rights to new_admin.
    /// Requires current_admin authorization.
    /// Emits AdminTransferred event.
    pub fn transfer_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Pauses market creation. Existing markets are unaffected and continue operating.
    /// Requires admin authorization.
    /// Emergency use only — document reason in the transaction memo.
    pub fn pause_factory(env: Env, admin: Address) -> Result<(), ContractError> {
        todo!()
    }

    /// Re-enables market creation after a pause.
    /// Requires admin authorization.
    pub fn unpause_factory(env: Env, admin: Address) -> Result<(), ContractError> {
        todo!()
    }

    /// Returns true if the factory is currently paused.
    pub fn is_paused(env: Env) -> bool {
        todo!()
    }

    /// Updates the default MarketConfig applied to new markets.
    /// Does NOT retroactively change existing markets.
    /// Requires admin authorization.
    pub fn update_default_config(
        env: Env,
        admin: Address,
        new_config: MarketConfig,
    ) -> Result<(), ContractError> {
        todo!()
    }
}
