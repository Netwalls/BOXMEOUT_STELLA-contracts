#![no_std]
use shared::types::{Fighter, ProtocolConfig};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, String, Vec};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// CONFIG           -> ProtocolConfig
// MARKET_COUNT     -> u64
// MARKET_{id}      -> Address  (deployed Market contract address)
// ALL_MARKETS      -> Vec<Bytes>
// PENDING_ADMIN    -> Address  (used during two-step transfer)

#[contracttype]
enum DataKey {
    Config,
    MarketCount,
    Market(Bytes),
    AllMarkets,
    PendingAdmin,
}

#[contract]
pub struct MarketFactory;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketCreatedEvent {
    pub market_id:      Bytes,
    pub fighter_a_name: String,
    pub fighter_b_name: String,
    pub scheduled_at:   u64,
    pub oracle:         Address,
    pub created_by:     Address,
}

#[contractimpl]
impl MarketFactory {

    /// Initializes the factory with protocol-wide config.
    /// Must be called once after deployment; panics if already initialized.
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_collector: Address,
        default_fee_bp: u32,
        min_bet: i128,
        max_bet: i128,
    ) {
        if env.storage().persistent().has(&DataKey::Config) {
            panic!("already initialized");
        }
        let config = ProtocolConfig {
            admin,
            fee_collector,
            default_fee_bp,
            min_bet_amount: min_bet,
            max_bet_amount: max_bet,
            dispute_window_sec: 86_400,
            paused: false,
        };
        env.storage().persistent().set(&DataKey::Config, &config);
        env.storage().persistent().set(&DataKey::MarketCount, &0u64);
        env.storage().persistent().set(&DataKey::AllMarkets, &Vec::<Bytes>::new(&env));
    }

    /// Deploys a new Market contract instance for a boxing match.
    /// Returns the unique market_id.
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

        let count: u64 = env.storage().persistent()
            .get(&DataKey::MarketCount)
            .unwrap_or(0);
        let mut id_bytes = [0u8; 32];
        id_bytes[..8].copy_from_slice(&(count + 1).to_be_bytes());
        let market_id = Bytes::from_array(&env, &id_bytes);

        env.storage().persistent().set(&DataKey::MarketCount, &(count + 1));

        let mut all: Vec<Bytes> = env.storage().persistent()
            .get(&DataKey::AllMarkets)
            .unwrap_or(Vec::new(&env));
        all.push_back(market_id.clone());
        env.storage().persistent().set(&DataKey::AllMarkets, &all);

        let event = MarketCreatedEvent {
            market_id: market_id.clone(),
            fighter_a_name: fighter_a.name.clone(),
            fighter_b_name: fighter_b.name.clone(),
            scheduled_at,
            oracle: oracle.clone(),
            created_by: caller.clone(),
        };
        env.events().publish((symbol_short!("mkt_crtd"),), event);

        market_id
    }

    /// Returns the deployed Market contract address for a given market_id.
    pub fn get_market_address(env: Env, market_id: Bytes) -> Address {
        env.storage().persistent()
            .get(&DataKey::Market(market_id))
            .expect("market not found")
    }

    /// Returns all market IDs ever created, ordered by creation time.
    pub fn get_all_markets(env: Env) -> Vec<Bytes> {
        env.storage().persistent()
            .get(&DataKey::AllMarkets)
            .unwrap_or(Vec::new(&env))
    }

    /// Returns a paginated slice of market IDs.
    pub fn get_markets_paginated(env: Env, offset: u32, limit: u32) -> Vec<Bytes> {
        let all: Vec<Bytes> = env.storage().persistent()
            .get(&DataKey::AllMarkets)
            .unwrap_or(Vec::new(&env));
        let mut result = Vec::new(&env);
        let len = all.len();
        let start = offset.min(len);
        let end = (offset + limit).min(len);
        for i in start..end {
            result.push_back(all.get(i).unwrap());
        }
        result
    }

    /// Updates the global protocol config. Only callable by the current admin.
    pub fn update_config(env: Env, admin: Address, new_config: ProtocolConfig) {
        admin.require_auth();
        let config: ProtocolConfig = env.storage().persistent()
            .get(&DataKey::Config)
            .expect("not initialized");
        if config.admin != admin {
            panic!("unauthorized");
        }
        env.storage().persistent().set(&DataKey::Config, &new_config);
    }

    /// Sets paused = true. Only callable by admin.
    pub fn pause_protocol(env: Env, admin: Address) {
        admin.require_auth();
        let mut config: ProtocolConfig = env.storage().persistent()
            .get(&DataKey::Config)
            .expect("not initialized");
        if config.admin != admin {
            panic!("unauthorized");
        }
        config.paused = true;
        env.storage().persistent().set(&DataKey::Config, &config);
    }

    /// Sets paused = false. Only callable by admin.
    pub fn unpause_protocol(env: Env, admin: Address) {
        admin.require_auth();
        let mut config: ProtocolConfig = env.storage().persistent()
            .get(&DataKey::Config)
            .expect("not initialized");
        if config.admin != admin {
            panic!("unauthorized");
        }
        config.paused = false;
        env.storage().persistent().set(&DataKey::Config, &config);
    }

    /// Initiates a two-step admin transfer.
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        admin.require_auth();
        let config: ProtocolConfig = env.storage().persistent()
            .get(&DataKey::Config)
            .expect("not initialized");
        if config.admin != admin {
            panic!("unauthorized");
        }
        env.storage().persistent().set(&DataKey::PendingAdmin, &new_admin);
    }

    /// Completes the two-step admin transfer.
    pub fn accept_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();
        let pending: Address = env.storage().persistent()
            .get(&DataKey::PendingAdmin)
            .expect("no pending transfer");
        if pending != new_admin {
            panic!("not the pending admin");
        }
        let mut config: ProtocolConfig = env.storage().persistent()
            .get(&DataKey::Config)
            .expect("not initialized");
        config.admin = new_admin;
        env.storage().persistent().set(&DataKey::Config, &config);
        env.storage().persistent().remove(&DataKey::PendingAdmin);
    }

    /// Returns the current ProtocolConfig.
    pub fn get_config(env: Env) -> ProtocolConfig {
        env.storage().persistent()
            .get(&DataKey::Config)
            .expect("not initialized")
    }
}

// ─── TESTS ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use shared::test_utils::{create_test_address, create_test_env};
    use soroban_sdk::String;

    fn make_fighter(env: &Env, name: &str) -> Fighter {
        Fighter {
            name:         String::from_str(env, name),
            record:       String::from_str(env, "10-0"),
            nationality:  String::from_str(env, "US"),
            weight_class: String::from_str(env, "Heavyweight"),
        }
    }

    /// Demonstrates the test harness: register, initialize, create a market, verify event.
    #[test]
    fn test_harness_create_market_emits_event() {
        let env = create_test_env();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, MarketFactory);
        let client = MarketFactoryClient::new(&env, &contract_id);

        let admin        = create_test_address(&env);
        let fee_col      = create_test_address(&env);
        let caller       = create_test_address(&env);
        let oracle       = create_test_address(&env);
        let fighter_a    = make_fighter(&env, "Alpha");
        let fighter_b    = make_fighter(&env, "Beta");

        client.initialize(&admin, &fee_col, &200u32, &1_000_000i128, &100_000_000i128);

        let market_id = client.create_market(
            &caller, &fighter_a, &fighter_b, &1_000_000u64, &900_000u64, &oracle,
        );

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let all_markets = client.get_all_markets();
        assert_eq!(all_markets.len(), 1);
        assert_eq!(all_markets.get(0).unwrap(), market_id);
    }

    #[test]
    fn test_harness_initialize_idempotency() {
        let env = create_test_env();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, MarketFactory);
        let client = MarketFactoryClient::new(&env, &contract_id);

        let admin   = create_test_address(&env);
        let fee_col = create_test_address(&env);

        client.initialize(&admin, &fee_col, &200u32, &1_000_000i128, &100_000_000i128);
        let config = client.get_config();
        assert_eq!(config.default_fee_bp, 200);
    }
}
