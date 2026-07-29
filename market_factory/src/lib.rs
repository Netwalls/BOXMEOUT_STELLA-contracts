#![no_std]
//! ============================================================
//! BOXMEOUT — MarketFactory Contract
//! Deploys and tracks Market contract instances.
//! ============================================================
use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, Map, String, Vec};

use shared::{errors::ContractError, types::MarketInfo};

// Storage keys for persistent state
const ADMIN: &str = "ADMIN";
const MARKET_WASM_HASH: &str = "MARKET_WASM_HASH";
const TREASURY: &str = "TREASURY";
const PAUSED: &str = "PAUSED";
const MARKET_COUNT_KEY: &str = "MARKET_COUNT";
const MARKET_MAP: &str = "MARKET_MAP";
const ALL_MARKETS_KEY: &str = "ALL_MARKETS";

#[contract]
pub struct MarketFactory;

#[contractimpl]
impl MarketFactory {
    /// One-time setup. Stores the admin, the Market contract's wasm hash
    /// (used for all future deployments), and the treasury address.
    ///
    /// # Errors
    /// Returns `ContractError::AlreadyInitialized` if called more than once.
    pub fn initialize(
        env: Env,
        admin: Address,
        market_wasm_hash: BytesN<32>,
        treasury: Address,
    ) -> Result<(), ContractError> {
        if env.storage().persistent().has(&ADMIN) {
            return Err(ContractError::AlreadyInitialized);
        }

        env.storage().persistent().set(&ADMIN, &admin);
        env.storage().persistent().set(&MARKET_WASM_HASH, &market_wasm_hash);
        env.storage().persistent().set(&TREASURY, &treasury);
        env.storage().persistent().set(&PAUSED, &false);
        env.storage().persistent().set(&MARKET_COUNT_KEY, &0u64);
        env.storage()
            .persistent()
            .set(&ALL_MARKETS_KEY, &Vec::<Bytes>::new(&env));
        env.storage()
            .persistent()
            .set(&MARKET_MAP, &Map::<Bytes, MarketInfo>::new(&env));

        Ok(())
    }

    /// Updates the Market wasm hash used for new deployments.
    ///
    /// Only the protocol admin can call this. Only affects markets deployed
    /// after this call — already-deployed Market instances keep running the
    /// wasm code they were originally deployed with.
    pub fn upgrade_market_wasm(
        env: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
    ) {
        admin.require_auth();

        let config: ProtocolConfig = env.storage().persistent()
            .get(&CONFIG_KEY)
            .expect("not initialized");

    /// Returns the stored Market contract wasm hash.
    pub fn get_market_wasm_hash(env: Env) -> BytesN<32> {
        env.storage()
            .persistent()
            .get(&MARKET_WASM_HASH)
            .expect("not initialized")
    }

    /// Returns the stored treasury address.
    pub fn get_treasury(env: Env) -> Address {
        env.storage().persistent().get(&TREASURY).expect("not initialized")
    }

    /// Deploys a new Market contract instance and registers its `MarketInfo`.
    ///
    /// # Errors
    /// Returns `ContractError::InvalidTimestamp` if `end_time` is in the
    /// past, or `lock_time` is after `end_time`.
    pub fn create_market(
        env: Env,
        caller: Address,
        fighter_a: String,
        fighter_b: String,
        oracle: Address,
        lock_time: u64,
        end_time: u64,
    ) -> Result<Bytes, ContractError> {
        caller.require_auth();

        let now = env.ledger().timestamp();
        if end_time <= now || lock_time > end_time {
            return Err(ContractError::InvalidTimestamp);
        }

        let count: u64 = env.storage().persistent().get(&MARKET_COUNT_KEY).unwrap_or(0);

        // Generate a collision-resistant market_id from the creation nonce,
        // both fighter names, and the scheduled end_time.
        let mut id_bytes = [0u8; 32];
        id_bytes[0..8].copy_from_slice(&count.to_le_bytes());
        for (i, byte) in fighter_a.to_bytes().iter().take(8).enumerate() {
            id_bytes[8 + i] ^= byte;
        }
        for (i, byte) in fighter_b.to_bytes().iter().take(8).enumerate() {
            id_bytes[16 + i] ^= byte;
        }
        id_bytes[24..32].copy_from_slice(&end_time.to_le_bytes());
        let market_id = Bytes::from_array(&env, &id_bytes);

        let wasm_hash: BytesN<32> = env
            .storage()
            .persistent()
            .get(&MARKET_WASM_HASH)
            .expect("not initialized");

        let salt = BytesN::from_array(&env, &id_bytes);
        let market_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(wasm_hash, ());

        let info = MarketInfo {
            market_id: market_id.clone(),
            market_address,
            creator: caller,
            fighter_a: fighter_a.clone(),
            fighter_b: fighter_b.clone(),
            oracle: oracle.clone(),
            lock_time,
            end_time,
            created_at: now,
        };

        let mut market_map: Map<Bytes, MarketInfo> = env
            .storage()
            .persistent()
            .get(&MARKET_MAP)
            .unwrap_or_else(|| Map::new(&env));
        market_map.set(market_id.clone(), info.clone());
        env.storage().persistent().set(&MARKET_MAP, &market_map);

        let mut all_markets: Vec<Bytes> = env
            .storage()
            .persistent()
            .get(&ALL_MARKETS_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        all_markets.push_back(market_id.clone());
        env.storage().persistent().set(&ALL_MARKETS_KEY, &all_markets);

        env.storage()
            .persistent()
            .set(&MARKET_COUNT_KEY, &(count + 1));

        env.events().publish(("market_created",), info);

        Ok(market_id)
    }

    /// Read-only lookup of a single market by ID. Does not mutate state.
    pub fn get_market(env: Env, market_id: Bytes) -> Option<MarketInfo> {
        let map: Map<Bytes, MarketInfo> = env
            .storage()
            .persistent()
            .get(&MARKET_MAP)
            .unwrap_or_else(|| Map::new(&env));
        map.get(market_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    // A minimal, dependency-free contract wasm used only to exercise the real
    // `env.deployer()...deploy_v2()` path in tests. It is not the production
    // Market contract; it just needs to be *some* valid, deployable wasm.
    const DUMMY_WASM: &[u8] = include_bytes!("../test_fixtures/dummy.wasm");

    fn setup() -> (Env, MarketFactoryClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(MarketFactory, ());
        let client = MarketFactoryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);

        (env, client, admin, treasury)
    }

    fn init(env: &Env, client: &MarketFactoryClient<'static>, admin: &Address, treasury: &Address) {
        let wasm_hash = env.deployer().upload_contract_wasm(DUMMY_WASM);
        client.initialize(admin, &wasm_hash, treasury);
    }

    fn create_default_market(env: &Env, client: &MarketFactoryClient<'static>) -> Bytes {
        let caller = Address::generate(env);
        let oracle = Address::generate(env);
        let now = env.ledger().timestamp();
        client.create_market(
            &caller,
            &String::from_str(env, "Fighter A"),
            &String::from_str(env, "Fighter B"),
            &oracle,
            &(now + 100),
            &(now + 200),
        )
    }

    // ── C-02: initialize ──────────────────────────────────────────────────

    #[test]
    fn initialize_stores_admin_wasm_hash_and_treasury() {
        let (env, client, admin, treasury) = setup();
        init(&env, &client, &admin, &treasury);

        let wasm_hash = env.deployer().upload_contract_wasm(DUMMY_WASM);
        assert_eq!(client.get_admin(), admin);
        assert_eq!(client.get_treasury(), treasury);
        assert_eq!(client.get_market_wasm_hash(), wasm_hash);
    }

    #[test]
    fn initialize_twice_fails() {
        let (env, client, admin, treasury) = setup();
        init(&env, &client, &admin, &treasury);

        let wasm_hash = env.deployer().upload_contract_wasm(DUMMY_WASM);
        let result = client.try_initialize(&admin, &wasm_hash, &treasury);
        assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
    }

    // ── C-03: create_market ───────────────────────────────────────────────

    #[test]
    fn create_market_happy_path() {
        let (env, client, admin, treasury) = setup();
        init(&env, &client, &admin, &treasury);

        let market_id = create_default_market(&env, &client);
        let info = client.get_market(&market_id).unwrap();

        assert_eq!(info.market_id, market_id);
        assert_eq!(info.fighter_a, String::from_str(&env, "Fighter A"));
        assert_eq!(info.fighter_b, String::from_str(&env, "Fighter B"));
    }

    #[test]
    fn create_market_rejects_end_time_in_the_past() {
        let (env, client, admin, treasury) = setup();
        init(&env, &client, &admin, &treasury);

        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let now = env.ledger().timestamp();

        let result = client.try_create_market(
            &caller,
            &String::from_str(&env, "Fighter A"),
            &String::from_str(&env, "Fighter B"),
            &oracle,
            &now,
            &now, // end_time == now, not in the future
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidTimestamp)));
    }

    #[test]
    fn create_market_rejects_lock_time_after_end_time() {
        let (env, client, admin, treasury) = setup();
        init(&env, &client, &admin, &treasury);

        let caller = Address::generate(&env);
        let oracle = Address::generate(&env);
        let now = env.ledger().timestamp();

        let result = client.try_create_market(
            &caller,
            &String::from_str(&env, "Fighter A"),
            &String::from_str(&env, "Fighter B"),
            &oracle,
            &(now + 200), // lock_time
            &(now + 100), // end_time, before lock_time
        );
        assert_eq!(result, Err(Ok(ContractError::InvalidTimestamp)));
    }

    // ── C-04: get_market ──────────────────────────────────────────────────

    #[test]
    fn get_market_unknown_id_returns_none() {
        let (env, client, admin, treasury) = setup();
        init(&env, &client, &admin, &treasury);

        let unknown_id = Bytes::from_array(&env, &[9u8; 32]);
        assert_eq!(client.get_market(&unknown_id), None);
    }

    #[test]
    fn get_market_known_id_returns_some() {
        let (env, client, admin, treasury) = setup();
        init(&env, &client, &admin, &treasury);

        let market_id = create_default_market(&env, &client);
        assert!(client.get_market(&market_id).is_some());
    }
}
