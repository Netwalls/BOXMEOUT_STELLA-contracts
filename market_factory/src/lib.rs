#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Vec};
use crate::types::{Fighter, ProtocolConfig};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// CONFIG           -> ProtocolConfig
// MARKET_COUNT     -> u64
// MARKET_{id}      -> Address  (deployed Market contract address)
// ALL_MARKETS      -> Vec<Bytes>
// PENDING_ADMIN    -> Address  (used during two-step transfer)

#[contract]
pub struct MarketFactory;

#[contractimpl]
impl MarketFactory {

    /// Initializes the factory with protocol-wide config.
    /// Must be called once after deployment; panics if already initialized.
    /// Stores ProtocolConfig in persistent contract storage.
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_collector: Address,
        default_fee_bp: u32,
        min_bet: i128,
        max_bet: i128,
    ) {
        todo!("implement: store ProtocolConfig, set MARKET_COUNT = 0, panic if already initialized")
    }

    /// Deploys a new Market contract instance for a boxing match.
    /// Validates: betting_ends_at < scheduled_at, fighters non-empty,
    /// scheduled_at is in the future, protocol is not paused.
    /// Increments MARKET_COUNT, appends market_id to ALL_MARKETS.
    /// Emits MarketCreated event.
    /// Returns the unique market_id (hash of fighter names + scheduled_at + nonce).
    pub fn create_market(
        env: Env,
        caller: Address,
        fighter_a: Fighter,
        fighter_b: Fighter,
        scheduled_at: u64,
        betting_ends_at: u64,
        oracle: Address,
    ) -> Bytes {
        todo!("implement: validate inputs, deploy Market contract, store address, emit event, return market_id")
    }

    /// Returns the deployed Market contract address for a given market_id.
    /// Panics with a descriptive error if market_id does not exist.
    pub fn get_market_address(env: Env, market_id: Bytes) -> Address {
        todo!("implement: read MARKET_{{id}} from storage, panic if missing")
    }

    /// Returns all market IDs ever created, ordered by creation time.
    /// Used by the backend indexer to enumerate all markets.
    pub fn get_all_markets(env: Env) -> Vec<Bytes> {
        todo!("implement: read ALL_MARKETS from storage, return vec")
    }

    /// Returns a paginated slice of market IDs.
    /// Useful for frontend browsing without loading the full list.
    pub fn get_markets_paginated(env: Env, offset: u32, limit: u32) -> Vec<Bytes> {
        todo!("implement: slice ALL_MARKETS from offset to offset+limit")
    }

    /// Updates the global protocol config (fees, limits, admin).
    /// Only callable by the current admin address.
    /// Emits ConfigUpdated event.
    pub fn update_config(env: Env, admin: Address, new_config: ProtocolConfig) {
        todo!("implement: require_auth(admin), validate new_config, store, emit event")
    }

    /// Sets paused = true in ProtocolConfig.
    /// Blocks new market creation and new bets across all markets.
    /// Only callable by admin.
    pub fn pause_protocol(env: Env, admin: Address) {
        todo!("implement: require_auth(admin), set paused=true, emit ProtocolPaused event")
    }

    /// Sets paused = false in ProtocolConfig.
    /// Restores normal operation.
    /// Only callable by admin.
    pub fn unpause_protocol(env: Env, admin: Address) {
        todo!("implement: require_auth(admin), set paused=false, emit ProtocolUnpaused event")
    }

    /// Initiates a two-step admin transfer.
    /// Stores new_admin as PENDING_ADMIN. Does NOT transfer immediately.
    /// Prevents accidental lockout — new_admin must call accept_admin() to confirm.
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        todo!("implement: require_auth(admin), store PENDING_ADMIN, emit AdminTransferInitiated")
    }

    /// Completes the two-step admin transfer.
    /// Caller must match PENDING_ADMIN. Sets new admin in ProtocolConfig.
    pub fn accept_admin(env: Env, new_admin: Address) {
        todo!("implement: require_auth(new_admin), check matches PENDING_ADMIN, update config, clear PENDING_ADMIN")
    }

    /// Returns the current ProtocolConfig.
    /// Read-only — callable by anyone.
    pub fn get_config(env: Env) -> ProtocolConfig {
        todo!("implement: read CONFIG from storage and return")
    }
}
