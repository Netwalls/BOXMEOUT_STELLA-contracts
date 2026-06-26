#![no_std]
mod types;

use crate::types::{Fighter, ProtocolConfig};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, String, Vec};

const CONFIG_KEY: &str = "CONFIG";
const MARKET_COUNT_KEY: &str = "MARKET_COUNT";
const ALL_MARKETS_KEY: &str = "ALL_MARKETS";
const PENDING_ADMIN_KEY: &str = "PENDING_ADMIN";

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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigUpdatedEvent {
    pub admin: Address,
    pub default_fee_bp: u32,
    pub min_bet_amount: i128,
    pub max_bet_amount: i128,
    pub dispute_window_sec: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolPausedEvent {
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolUnpausedEvent {
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferInitiatedEvent {
    pub admin: Address,
    pub new_admin: Address,
}

#[contract]
pub struct MarketFactory;

#[contractimpl]
impl MarketFactory {

    pub fn initialize(
        env: Env,
        admin: Address,
        fee_collector: Address,
        default_fee_bp: u32,
        min_bet: i128,
        max_bet: i128,
    ) {
        assert!(!env.storage().persistent().has(&CONFIG_KEY), "already initialized");

        let config = ProtocolConfig {
            admin,
            fee_collector,
            default_fee_bp,
            min_bet_amount: min_bet,
            max_bet_amount: max_bet,
            dispute_window_sec: 86400,
            paused: false,
        };
        env.storage().persistent().set(&CONFIG_KEY, &config);
        env.storage().persistent().set(&MARKET_COUNT_KEY, &0u64);
        let all_markets: Vec<Bytes> = Vec::new(&env);
        env.storage().persistent().set(&ALL_MARKETS_KEY, &all_markets);
    }

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

        let config: ProtocolConfig = env.storage().persistent()
            .get(&CONFIG_KEY).expect("not initialized");

        assert!(!config.paused, "protocol is paused");
        assert!(scheduled_at > env.ledger().timestamp(), "scheduled_at must be in the future");
        assert!(betting_ends_at < scheduled_at, "betting_ends_at must be before scheduled_at");
        assert!(!fighter_a.name.is_empty(), "fighter_a name cannot be empty");
        assert!(!fighter_b.name.is_empty(), "fighter_b name cannot be empty");

        let count: u64 = env.storage().persistent()
            .get(&MARKET_COUNT_KEY).unwrap_or(0u64);
        let new_count = count + 1;
        let mut id_bytes = [0u8; 32];
        id_bytes[..8].copy_from_slice(&new_count.to_be_bytes());
        let market_id = Bytes::from_array(&env, &id_bytes);

        let mut all_markets: Vec<Bytes> = env.storage().persistent()
            .get(&ALL_MARKETS_KEY).unwrap_or(Vec::new(&env));
        all_markets.push_back(market_id.clone());
        env.storage().persistent().set(&ALL_MARKETS_KEY, &all_markets);
        env.storage().persistent().set(&MARKET_COUNT_KEY, &new_count);

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

    pub fn get_market_address(env: Env, market_id: Bytes) -> Address {
        env.storage().persistent()
            .get(&market_id)
            .expect("market not found")
    }

    pub fn get_all_markets(env: Env) -> Vec<Bytes> {
        env.storage().persistent()
            .get(&ALL_MARKETS_KEY)
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_markets_paginated(env: Env, offset: u32, limit: u32) -> Vec<Bytes> {
        let all: Vec<Bytes> = env.storage().persistent()
            .get(&ALL_MARKETS_KEY)
            .unwrap_or(Vec::new(&env));
        let total = all.len();
        if offset >= total {
            return Vec::new(&env);
        }
        let end = (offset + limit).min(total);
        let mut result: Vec<Bytes> = Vec::new(&env);
        for i in offset..end {
            result.push_back(all.get(i).unwrap());
        }
        result
    }

    pub fn update_config(env: Env, admin: Address, new_config: ProtocolConfig) {
        admin.require_auth();

        let mut config: ProtocolConfig = env.storage().persistent()
            .get(&CONFIG_KEY).expect("not initialized");

        assert_eq!(config.admin, admin, "unauthorized");

        config.fee_collector = new_config.fee_collector;
        config.default_fee_bp = new_config.default_fee_bp;
        config.min_bet_amount = new_config.min_bet_amount;
        config.max_bet_amount = new_config.max_bet_amount;
        config.dispute_window_sec = new_config.dispute_window_sec;

        env.storage().persistent().set(&CONFIG_KEY, &config);

        env.events().publish((symbol_short!("config_updtd"),), ConfigUpdatedEvent {
            admin: admin.clone(),
            default_fee_bp: config.default_fee_bp,
            min_bet_amount: config.min_bet_amount,
            max_bet_amount: config.max_bet_amount,
            dispute_window_sec: config.dispute_window_sec,
        });
    }

    pub fn pause_protocol(env: Env, admin: Address) {
        admin.require_auth();

        let mut config: ProtocolConfig = env.storage().persistent()
            .get(&CONFIG_KEY).expect("not initialized");

        assert_eq!(config.admin, admin, "unauthorized");
        config.paused = true;
        env.storage().persistent().set(&CONFIG_KEY, &config);

        env.events().publish((symbol_short!("protocol_ps"),), ProtocolPausedEvent {
            admin,
        });
    }

    pub fn unpause_protocol(env: Env, admin: Address) {
        admin.require_auth();

        let mut config: ProtocolConfig = env.storage().persistent()
            .get(&CONFIG_KEY).expect("not initialized");

        assert_eq!(config.admin, admin, "unauthorized");
        config.paused = false;
        env.storage().persistent().set(&CONFIG_KEY, &config);

        env.events().publish((symbol_short!("protocol_up"),), ProtocolUnpausedEvent {
            admin,
        });
    }

    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        admin.require_auth();

        let config: ProtocolConfig = env.storage().persistent()
            .get(&CONFIG_KEY).expect("not initialized");

        assert_eq!(config.admin, admin, "unauthorized");

        env.storage().persistent().set(&PENDING_ADMIN_KEY, &new_admin);

        env.events().publish((symbol_short!("admin_trans"),), AdminTransferInitiatedEvent {
            admin,
            new_admin,
        });
    }

    pub fn accept_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();

        let pending: Address = env.storage().persistent()
            .get(&PENDING_ADMIN_KEY).expect("no pending admin transfer");

        assert_eq!(pending, new_admin, "not the pending admin");

        let mut config: ProtocolConfig = env.storage().persistent()
            .get(&CONFIG_KEY).expect("not initialized");

        config.admin = new_admin.clone();
        env.storage().persistent().set(&CONFIG_KEY, &config);
        env.storage().persistent().remove(&PENDING_ADMIN_KEY);
    }

    pub fn get_config(env: Env) -> ProtocolConfig {
        env.storage().persistent()
            .get(&CONFIG_KEY).expect("not initialized")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, MarketFactoryClient, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, MarketFactory);
        let client = MarketFactoryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        (env, client, admin)
    }

    fn init(client: &MarketFactoryClient, admin: &Address) {
        client.initialize(
            admin,
            &Address::generate(&client.env),
            &200u32,
            &100i128,
            &10000i128,
        );
    }

    fn sample_fighter_a(env: &Env) -> Fighter {
        Fighter {
            name: String::from_str(env, "Alpha"),
            record: String::from_str(env, "10-0"),
            nationality: String::from_str(env, "US"),
            weight_class: String::from_str(env, "Heavyweight"),
        }
    }

    fn sample_fighter_b(env: &Env) -> Fighter {
        Fighter {
            name: String::from_str(env, "Beta"),
            record: String::from_str(env, "9-1"),
            nationality: String::from_str(env, "CA"),
            weight_class: String::from_str(env, "Heavyweight"),
        }
    }

    // ── initialize ─────────────────────────────────────────────────────────

    #[test]
    fn test_initialize_sets_config() {
        let (env, client, admin) = setup();
        client.initialize(&admin, &Address::generate(&env), &200u32, &100i128, &10000i128);

        let config = client.get_config();
        assert_eq!(config.admin, admin);
        assert_eq!(config.default_fee_bp, 200);
        assert_eq!(config.min_bet_amount, 100);
        assert_eq!(config.max_bet_amount, 10000);
        assert!(!config.paused);
    }

    #[test]
    fn test_initialize_panics_if_called_twice() {
        let (env, client, admin) = setup();
        let fee_collector = Address::generate(&env);
        client.initialize(&admin, &fee_collector, &200u32, &100i128, &10000i128);

        let result = std::panic::catch_unwind(|| {
            client.initialize(&admin, &fee_collector, &200u32, &100i128, &10000i128);
        });
        assert!(result.is_err(), "double initialize should panic");
    }

    #[test]
    fn test_initialize_stores_empty_market_list() {
        let (env, client, admin) = setup();
        client.initialize(&admin, &Address::generate(&env), &200u32, &100i128, &10000i128);

        let markets = client.get_all_markets();
        assert_eq!(markets.len(), 0);
    }

    // ── create_market ───────────────────────────────────────────────────────

    #[test]
    fn test_create_market_emits_event() {
        let (env, client, admin) = setup();
        init(&client, &admin);

        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let future = env.ledger().timestamp() + 1000;
        let market_id = client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future - 100), &oracle);

        let events = env.events().all();
        let event = events.get(0).unwrap();
        let topics = event.0;
        assert_eq!(topics.get(0).unwrap(), symbol_short!("market_created"));

        let data: MarketCreatedEvent = event.1.try_into().unwrap();
        assert_eq!(data.market_id, market_id);
        assert_eq!(data.fighter_a_name, String::from_str(&env, "Alpha"));
        assert_eq!(data.fighter_b_name, String::from_str(&env, "Beta"));
        assert_eq!(data.scheduled_at, future);
        assert_eq!(data.oracle, oracle);
        assert_eq!(data.created_by, caller);
    }

    #[test]
    fn test_create_market_increments_market_list() {
        let (env, client, admin) = setup();
        init(&client, &admin);

        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let future = env.ledger().timestamp() + 1000;

        let id1 = client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future - 100), &oracle);
        let id2 = client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &(future + 2000), &(future + 1900), &oracle);

        let all = client.get_all_markets();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get(0).unwrap(), id1);
        assert_eq!(all.get(1).unwrap(), id2);
    }

    #[test]
    fn test_create_market_panics_when_paused() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        client.pause_protocol(&admin);

        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let future = env.ledger().timestamp() + 1000;

        let result = std::panic::catch_unwind(|| {
            client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future - 100), &oracle);
        });
        assert!(result.is_err(), "create_market should panic when paused");
    }

    #[test]
    fn test_create_market_panics_when_scheduled_at_in_past() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let past = env.ledger().timestamp() - 100;

        let result = std::panic::catch_unwind(|| {
            client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &past, &(past - 100), &oracle);
        });
        assert!(result.is_err(), "create_market should panic when scheduled_at is in the past");
    }

    #[test]
    fn test_create_market_panics_when_betting_ends_after_scheduled() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let future = env.ledger().timestamp() + 1000;

        let result = std::panic::catch_unwind(|| {
            client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future + 100), &oracle);
        });
        assert!(result.is_err(), "create_market should panic when betting_ends_at >= scheduled_at");
    }

    #[test]
    fn test_create_market_panics_with_empty_fighter_name() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let future = env.ledger().timestamp() + 1000;
        let empty_fighter = Fighter {
            name: String::from_str(&env, ""),
            record: String::from_str(&env, "0-0"),
            nationality: String::from_str(&env, "US"),
            weight_class: String::from_str(&env, "Heavyweight"),
        };

        let result = std::panic::catch_unwind(|| {
            client.create_market(&caller, &empty_fighter, &sample_fighter_b(&env), &future, &(future - 100), &oracle);
        });
        assert!(result.is_err(), "create_market should panic with empty fighter name");
    }

    // ── get_all_markets ─────────────────────────────────────────────────────

    #[test]
    fn test_get_all_markets_returns_all() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);

        for i in 0u64..3u64 {
            let future = env.ledger().timestamp() + 1000 + i * 2000;
            client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future - 100), &oracle);
        }

        let all = client.get_all_markets();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_all_markets_returns_empty_before_any_created() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let all = client.get_all_markets();
        assert_eq!(all.len(), 0);
    }

    // ── get_markets_paginated ───────────────────────────────────────────────

    #[test]
    fn test_get_markets_paginated_returns_slice() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);

        let mut ids = Vec::new(&env);
        for i in 0u64..10u64 {
            let future = env.ledger().timestamp() + 1000 + i * 2000;
            let id = client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future - 100), &oracle);
            ids.push_back(id);
        }

        let page = client.get_markets_paginated(&0u32, &5u32);
        assert_eq!(page.len(), 5);
        for i in 0..5 {
            assert_eq!(page.get(i).unwrap(), ids.get(i).unwrap());
        }
    }

    #[test]
    fn test_get_markets_paginated_returns_remainder() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);

        for i in 0u64..10u64 {
            let future = env.ledger().timestamp() + 1000 + i * 2000;
            client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future - 100), &oracle);
        }

        let page = client.get_markets_paginated(&8u32, &5u32);
        assert_eq!(page.len(), 2); // only indices 8,9 remain
    }

    #[test]
    fn test_get_markets_paginated_returns_empty_when_offset_exceeds_length() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let future = env.ledger().timestamp() + 1000;
        client.create_market(&caller, &sample_fighter_a(&env), &sample_fighter_b(&env), &future, &(future - 100), &oracle);

        let page = client.get_markets_paginated(&100u32, &5u32);
        assert_eq!(page.len(), 0);
    }

    // ── pause / unpause ─────────────────────────────────────────────────────

    #[test]
    fn test_pause_protocol_sets_paused() {
        let (env, client, admin) = setup();
        init(&client, &admin);

        client.pause_protocol(&admin);
        let config = client.get_config();
        assert!(config.paused);
    }

    #[test]
    fn test_pause_protocol_emits_event() {
        let (env, client, admin) = setup();
        init(&client, &admin);

        client.pause_protocol(&admin);

        let events = env.events().all();
        let event = events.get(events.len() - 1).unwrap();
        let topics = event.0;
        assert_eq!(topics.get(0).unwrap(), symbol_short!("protocol_ps"));
        let data: ProtocolPausedEvent = event.1.try_into().unwrap();
        assert_eq!(data.admin, admin);
    }

    #[test]
    fn test_unpause_protocol_clears_paused() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        client.pause_protocol(&admin);
        assert!(client.get_config().paused);

        client.unpause_protocol(&admin);
        assert!(!client.get_config().paused);
    }

    #[test]
    fn test_unpause_protocol_emits_event() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        client.pause_protocol(&admin);

        client.unpause_protocol(&admin);

        let events = env.events().all();
        let event = events.get(events.len() - 1).unwrap();
        let topics = event.0;
        assert_eq!(topics.get(0).unwrap(), symbol_short!("protocol_up"));
        let data: ProtocolUnpausedEvent = event.1.try_into().unwrap();
        assert_eq!(data.admin, admin);
    }

    #[test]
    fn test_pause_protocol_panics_if_not_admin() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let other = Address::generate(&env);

        let result = std::panic::catch_unwind(|| {
            client.pause_protocol(&other);
        });
        assert!(result.is_err(), "non-admin should not be able to pause");
    }

    #[test]
    fn test_unpause_protocol_panics_if_not_admin() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        client.pause_protocol(&admin);
        let other = Address::generate(&env);

        let result = std::panic::catch_unwind(|| {
            client.unpause_protocol(&other);
        });
        assert!(result.is_err(), "non-admin should not be able to unpause");
    }

    // ── transfer_admin / accept_admin ───────────────────────────────────────

    #[test]
    fn test_transfer_admin_stores_pending() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let new_admin = Address::generate(&env);

        client.transfer_admin(&admin, &new_admin);
        client.accept_admin(&new_admin);

        let config = client.get_config();
        assert_eq!(config.admin, new_admin);
    }

    #[test]
    fn test_transfer_admin_emits_event() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let new_admin = Address::generate(&env);

        client.transfer_admin(&admin, &new_admin);

        let events = env.events().all();
        let event = events.get(events.len() - 1).unwrap();
        let topics = event.0;
        assert_eq!(topics.get(0).unwrap(), symbol_short!("admin_trans"));
        let data: AdminTransferInitiatedEvent = event.1.try_into().unwrap();
        assert_eq!(data.admin, admin);
        assert_eq!(data.new_admin, new_admin);
    }

    #[test]
    fn test_accept_admin_completes_transfer() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let new_admin = Address::generate(&env);

        client.transfer_admin(&admin, &new_admin);
        client.accept_admin(&new_admin);

        let config = client.get_config();
        assert_eq!(config.admin, new_admin);
        // Old admin can no longer control
        let result = std::panic::catch_unwind(|| {
            client.pause_protocol(&admin);
        });
        assert!(result.is_err(), "old admin should not be able to pause after transfer");
    }

    #[test]
    fn test_accept_admin_panics_without_pending_transfer() {
        let (env, client, _admin) = setup();
        let new_admin = Address::generate(&env);

        let result = std::panic::catch_unwind(|| {
            client.accept_admin(&new_admin);
        });
        assert!(result.is_err(), "accept_admin should panic without pending transfer");
    }

    #[test]
    fn test_accept_admin_panics_if_not_pending_admin() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let new_admin = Address::generate(&env);
        let other = Address::generate(&env);

        client.transfer_admin(&admin, &new_admin);
        let result = std::panic::catch_unwind(|| {
            client.accept_admin(&other);
        });
        assert!(result.is_err(), "accept_admin should panic if caller is not the pending admin");
    }

    #[test]
    fn test_transfer_admin_panics_if_not_admin() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let other = Address::generate(&env);
        let new_admin = Address::generate(&env);

        let result = std::panic::catch_unwind(|| {
            client.transfer_admin(&other, &new_admin);
        });
        assert!(result.is_err(), "non-admin should not be able to transfer admin");
    }

    // ── update_config ───────────────────────────────────────────────────────

    #[test]
    fn test_update_config_changes_values() {
        let (env, client, admin) = setup();
        init(&client, &admin);

        let new_config = ProtocolConfig {
            admin: admin.clone(),
            fee_collector: Address::generate(&env),
            default_fee_bp: 500,
            min_bet_amount: 50,
            max_bet_amount: 50000,
            dispute_window_sec: 172800,
            paused: false,
        };
        client.update_config(&admin, &new_config);

        let config = client.get_config();
        assert_eq!(config.default_fee_bp, 500);
        assert_eq!(config.min_bet_amount, 50);
        assert_eq!(config.max_bet_amount, 50000);
        assert_eq!(config.dispute_window_sec, 172800);
    }

    #[test]
    fn test_update_config_emits_event() {
        let (env, client, admin) = setup();
        init(&client, &admin);

        let new_config = ProtocolConfig {
            admin: admin.clone(),
            fee_collector: Address::generate(&env),
            default_fee_bp: 500,
            min_bet_amount: 50,
            max_bet_amount: 50000,
            dispute_window_sec: 172800,
            paused: false,
        };
        client.update_config(&admin, &new_config);

        let events = env.events().all();
        let event = events.get(events.len() - 1).unwrap();
        let topics = event.0;
        assert_eq!(topics.get(0).unwrap(), symbol_short!("config_updtd"));
        let data: ConfigUpdatedEvent = event.1.try_into().unwrap();
        assert_eq!(data.default_fee_bp, 500);
        assert_eq!(data.min_bet_amount, 50);
        assert_eq!(data.max_bet_amount, 50000);
    }

    #[test]
    fn test_update_config_panics_if_not_initialized() {
        let (env, _client, _admin) = setup();
        let result = std::panic::catch_unwind(|| {
            let client = MarketFactoryClient::new(&env, &Bytes::from_array(&env, &[0u8; 32]));
            client.update_config(&Address::generate(&env), &ProtocolConfig {
                admin: Address::generate(&env),
                fee_collector: Address::generate(&env),
                default_fee_bp: 200,
                min_bet_amount: 100,
                max_bet_amount: 10000,
                dispute_window_sec: 86400,
                paused: false,
            });
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_update_config_panics_if_not_admin() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let other = Address::generate(&env);

        let result = std::panic::catch_unwind(|| {
            client.update_config(&other, &ProtocolConfig {
                admin: other.clone(),
                fee_collector: Address::generate(&env),
                default_fee_bp: 500,
                min_bet_amount: 50,
                max_bet_amount: 50000,
                dispute_window_sec: 172800,
                paused: false,
            });
        });
        assert!(result.is_err(), "non-admin should not be able to update config");
    }

    // ── get_config ──────────────────────────────────────────────────────────

    #[test]
    fn test_get_config_returns_config() {
        let (env, client, admin) = setup();
        init(&client, &admin);
        let config = client.get_config();
        assert_eq!(config.admin, admin);
    }

    #[test]
    fn test_get_config_panics_if_not_initialized() {
        let (env, client, _admin) = setup();
        let result = std::panic::catch_unwind(|| {
            client.get_config();
        });
        assert!(result.is_err(), "get_config should panic if not initialized");
    }
}
