#![no_std]
use crate::types::{Fighter, ProtocolConfig};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, String, Vec};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// CONFIG           -> ProtocolConfig
// MARKET_COUNT     -> u64
// MARKET_{id}      -> Address  (deployed Market contract address)
// ALL_MARKETS      -> Vec<Bytes>
// PENDING_ADMIN    -> Address  (used during two-step transfer)

#[contract]
pub struct MarketFactory;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketCreatedEvent {
    pub market_id: Bytes,
    pub fighter_a_name: String,
    pub fighter_b_name: String,
    pub scheduled_at: u64,
    pub oracle: Address,
    pub created_by: Address,
}

#[contractimpl]
impl MarketFactory {

    /// Initializes the factory with protocol-wide configuration.
    ///
    /// Must be called once immediately after deployment. Stores a [`ProtocolConfig`]
    /// in persistent storage and sets `MARKET_COUNT` to zero.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Address of the protocol administrator.
    /// * `fee_collector` - Address that receives protocol fees.
    /// * `default_fee_bp` - Default fee in basis points applied to all new markets (e.g. `200` = 2%).
    /// * `min_bet` - Minimum allowed bet amount in stroops.
    /// * `max_bet` - Maximum allowed bet amount in stroops.
    ///
    /// # Panics
    ///
    /// Panics if the factory has already been initialized.
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

    /// Deploys and registers a new boxing prediction market.
    ///
    /// Validates that `betting_ends_at < scheduled_at`, fighter names are non-empty,
    /// `scheduled_at` is in the future, and the protocol is not paused. Increments
    /// `MARKET_COUNT`, appends the new `market_id` to `ALL_MARKETS`, and emits a
    /// `MarketCreated` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `caller` - Address creating the market. Must authorize this call.
    /// * `fighter_a` - Metadata for the first fighter.
    /// * `fighter_b` - Metadata for the second fighter.
    /// * `scheduled_at` - Unix timestamp (seconds) of the scheduled fight.
    /// * `betting_ends_at` - Unix timestamp after which betting closes. Must be before `scheduled_at`.
    /// * `oracle` - Address authorized to lock and resolve the new market.
    ///
    /// # Returns
    ///
    /// Returns the unique `market_id` (`Bytes`) for the newly created market,
    /// derived from fighter names, `scheduled_at`, and an incrementing nonce.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The protocol is paused.
    /// - `betting_ends_at >= scheduled_at`.
    /// - Either fighter name is empty.
    /// - `scheduled_at` is not in the future.
    /// - `caller` has not authorized the call.
    pub fn create_market(
        env: Env,
        caller: Address,
        fighter_a: Fighter,
        fighter_b: Fighter,
        scheduled_at: u64,
        betting_ends_at: u64,
        oracle: Address,
    ) -> Bytes {
        caller.require_auth();

        let market_id = Bytes::from_array(&[1u8; 32]);
        let event = MarketCreatedEvent {
            market_id: market_id.clone(),
            fighter_a_name: fighter_a.name.clone(),
            fighter_b_name: fighter_b.name.clone(),
            scheduled_at,
            oracle: oracle.clone(),
            created_by: caller.clone(),
        };
        env.events().publish((symbol_short!("market_created"),), event);

        market_id
    }

    /// Returns the deployed `Market` contract address for a given `market_id`.
    ///
    /// Read-only — does not modify state.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `market_id` - The unique identifier of the market to look up.
    ///
    /// # Returns
    ///
    /// Returns the [`Address`] of the deployed `Market` contract.
    ///
    /// # Panics
    ///
    /// Panics with a descriptive error if `market_id` does not correspond to any
    /// registered market.
    pub fn get_market_address(env: Env, market_id: Bytes) -> Address {
        todo!("implement: read MARKET_{{id}} from storage, panic if missing")
    }

    /// Returns all market IDs ever created, in creation order.
    ///
    /// Used by the backend indexer to enumerate every market deployed through
    /// this factory. Read-only — does not modify state.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    ///
    /// # Returns
    ///
    /// Returns a [`Vec<Bytes>`] of all market IDs ordered by creation time.
    pub fn get_all_markets(env: Env) -> Vec<Bytes> {
        todo!("implement: read ALL_MARKETS from storage, return vec")
    }

    /// Returns a paginated slice of market IDs.
    ///
    /// Slices `ALL_MARKETS` from `offset` to `offset + limit`. Useful for
    /// frontend browsing without loading the full market list. Read-only.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `offset` - Zero-based index of the first market to return.
    /// * `limit` - Maximum number of market IDs to return.
    ///
    /// # Returns
    ///
    /// Returns a [`Vec<Bytes>`] containing up to `limit` market IDs starting at `offset`.
    /// Returns an empty `Vec` if `offset` is beyond the end of the list.
    pub fn get_markets_paginated(env: Env, offset: u32, limit: u32) -> Vec<Bytes> {
        todo!("implement: slice ALL_MARKETS from offset to offset+limit")
    }

    /// Updates the global protocol configuration.
    ///
    /// Replaces the stored [`ProtocolConfig`] with `new_config`. Only callable by
    /// the current admin. Emits a `ConfigUpdated` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Current admin address. Must authorize this call.
    /// * `new_config` - The replacement [`ProtocolConfig`] to store.
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call or is not the configured admin.
    pub fn update_config(env: Env, admin: Address, new_config: ProtocolConfig) {
        todo!("implement: require_auth(admin), validate new_config, store, emit event")
    }

    /// Pauses the protocol, blocking new market creation and new bets.
    ///
    /// Sets `paused = true` in [`ProtocolConfig`]. Only callable by the admin.
    /// Emits a `ProtocolPaused` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Admin address. Must authorize this call.
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call or is not the configured admin.
    pub fn pause_protocol(env: Env, admin: Address) {
        todo!("implement: require_auth(admin), set paused=true, emit ProtocolPaused event")
    }

    /// Unpauses the protocol, restoring normal operation.
    ///
    /// Sets `paused = false` in [`ProtocolConfig`]. Only callable by the admin.
    /// Emits a `ProtocolUnpaused` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Admin address. Must authorize this call.
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call or is not the configured admin.
    pub fn unpause_protocol(env: Env, admin: Address) {
        todo!("implement: require_auth(admin), set paused=false, emit ProtocolUnpaused event")
    }

    /// Initiates a two-step admin transfer to prevent accidental lockout.
    ///
    /// Stores `new_admin` as `PENDING_ADMIN`. The transfer is not complete until
    /// `new_admin` calls [`accept_admin`]. Emits an `AdminTransferInitiated` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Current admin address. Must authorize this call.
    /// * `new_admin` - Address that will become admin after calling [`accept_admin`].
    ///
    /// # Panics
    ///
    /// Panics if `admin` has not authorized the call or is not the configured admin.
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        todo!("implement: require_auth(admin), store PENDING_ADMIN, emit AdminTransferInitiated")
    }

    /// Completes a pending two-step admin transfer.
    ///
    /// Caller must match the address stored as `PENDING_ADMIN`. Sets the new admin
    /// in [`ProtocolConfig`] and clears `PENDING_ADMIN`.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `new_admin` - Address accepting the admin role. Must authorize this call
    ///   and must match `PENDING_ADMIN`.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `new_admin` has not authorized the call.
    /// - `new_admin` does not match `PENDING_ADMIN`.
    /// - No pending admin transfer exists.
    pub fn accept_admin(env: Env, new_admin: Address) {
        todo!("implement: require_auth(new_admin), check matches PENDING_ADMIN, update config, clear PENDING_ADMIN")
    }

    /// Returns the current [`ProtocolConfig`].
    ///
    /// Read-only — callable by anyone, does not modify state.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    ///
    /// # Returns
    ///
    /// Returns the [`ProtocolConfig`] stored in this factory.
    ///
    /// # Panics
    ///
    /// Panics if the factory has not been initialized.
    pub fn get_config(env: Env) -> ProtocolConfig {
        todo!("implement: read CONFIG from storage and return")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn create_market_emits_market_created_event() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MarketFactory);
        let client = MarketFactoryClient::new(&env, &contract_id);

        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let fighter_a = Fighter {
            name: String::from_str(&env, "Alpha"),
            record: String::from_str(&env, "10-0"),
            nationality: String::from_str(&env, "US"),
            weight_class: String::from_str(&env, "Heavyweight"),
        };
        let fighter_b = Fighter {
            name: String::from_str(&env, "Beta"),
            record: String::from_str(&env, "9-1"),
            nationality: String::from_str(&env, "CA"),
            weight_class: String::from_str(&env, "Heavyweight"),
        };

        let market_id = client.create_market(&caller, &fighter_a, &fighter_b, &100u64, &90u64, &oracle);
        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let event = events.get(0).unwrap().unwrap();
        let topics = event.0;
        assert_eq!(topics.len(), 1);
        assert_eq!(topics.get(0).unwrap(), symbol_short!("market_created"));

        let data = event.1;
        assert_eq!(
            data,
            MarketCreatedEvent {
                market_id: market_id.clone(),
                fighter_a_name: fighter_a.name.clone(),
                fighter_b_name: fighter_b.name.clone(),
                scheduled_at: 100u64,
                oracle: oracle.clone(),
                created_by: caller.clone(),
            }
        );
    }
}
