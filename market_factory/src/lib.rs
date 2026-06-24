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
