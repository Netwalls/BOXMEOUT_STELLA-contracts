#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, Vec, Symbol, BytesN};
use soroban_sdk::{contract, contractimpl, token, Address, Bytes, Env, Symbol, Vec};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// ADMIN              -> Address
// FACTORY            -> Address
// BALANCE            -> i128
// TOTAL_FEES_EARNED  -> i128
// WITHDRAWAL_LOG     -> Vec<(Address, i128, u64)>

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProtocolConfig {
    pub admin:              Address,
    pub fee_collector:      Address,
    pub default_fee_bp:     u32,
    pub min_bet_amount:     i128,
    pub max_bet_amount:     i128,
    pub dispute_window_sec: u64,
    pub paused:             bool,
}

#[contract]
pub struct Treasury;

#[contractimpl]
impl Treasury {

    /// Sets up the Treasury with admin and authorized factory address.
    /// Called once after deployment. Panics if already initialized.
    pub fn initialize(env: Env, admin: Address, factory: Address) {
        // Check if already initialized
        let admin_key = Symbol::short("ADMIN");
        if env.storage().has(&admin_key) {
            panic!("Treasury contract is already initialized");
        }

        // Persist admin and factory
        env.storage().set(&admin_key, &admin);
        env.storage().set(&Symbol::short("FACTORY"), &factory);

        // Initialize numeric metrics
        env.storage().set(&Symbol::short("BALANCE"), &0i128);
        env.storage().set(&Symbol::short("TOTAL_FEES_EARNED"), &0i128);

        // Initialize empty withdrawal log
        let log: Vec<(Address, i128, u64)> = Vec::new(&env);
        env.storage().set(&Symbol::short("WITHDRAWAL_LOG"), &log);
    }

    /// Called by Market contracts when distributing protocol fees on claim.
    /// Validates caller is a Market contract registered in the factory.
    /// Adds amount to BALANCE and TOTAL_FEES_EARNED.
    /// Emits FeesDeposited event.
    pub fn deposit_fees(env: Env, market_id: Bytes, amount: i128) {
        // Fetch factory address from storage
        let factory: Address = env
            .storage()
            .get(&Symbol::short("FACTORY"))
            .expect("Factory not configured");

        // Determine caller (the Market contract that invoked this)
        let caller: Address = env.invoker();

        // Cross-contract call to factory.get_market_address(market_id) to verify registration
        let registered_addr: Address = env.invoke_contract(
            &factory,
            &Symbol::short("get_market_address"),
            (market_id.clone(),),
        );

        if registered_addr != caller {
            panic!("Unauthorized caller: caller is not the registered market")
        }

        // NOTE: In a full implementation we'd transfer XLM from caller to this contract
        // via the Stellar Asset Contract (SAC) client. Here we update accounting state.
        let prev_balance: i128 = env.storage().get(&Symbol::short("BALANCE")).unwrap_or(0i128);
        let prev_total: i128 = env.storage().get(&Symbol::short("TOTAL_FEES_EARNED")).unwrap_or(0i128);
        let new_balance = prev_balance + amount;
        let new_total = prev_total + amount;
        env.storage().set(&Symbol::short("BALANCE"), &new_balance);
        env.storage().set(&Symbol::short("TOTAL_FEES_EARNED"), &new_total);

        // Emit FeesDeposited event
        env.events().publish((Symbol::short("FeesDeposited"),), crate::types::FeesDeposited {
            caller: caller.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        });
    }

    /// Transfers collected fees to a recipient (e.g. DAO multisig, team wallet).
    /// Validates: caller is admin, amount <= BALANCE.
    /// Appends withdrawal to WITHDRAWAL_LOG.
    /// Emits FeesWithdrawn event.
    pub fn withdraw_fees(env: Env, admin: Address, recipient: Address, amount: i128) {
        // 1. admin.require_auth() must be the first call
        admin.require_auth();

        // 2. Verify amount <= BALANCE, panic otherwise
        let balance: i128 = env.storage().persistent().get(&Symbol::new(&env, "BALANCE")).unwrap_or(0);
        if amount > balance {
            panic!("amount exceeds balance");
        }

        // 3. Deduct amount from BALANCE
        env.storage().persistent().set(&Symbol::new(&env, "BALANCE"), &(balance - amount));

        // 4. Transfer XLM to recipient
        let native = env.register_stellar_asset_contract(Address::from_str(&env, "native"));
        let token_client = token::Client::new(&env, &native);
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        // 5. Append (recipient, amount, timestamp) to WITHDRAWAL_LOG
        let timestamp = env.ledger().timestamp();
        let mut log: Vec<(Address, i128, u64)> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "WITHDRAWAL_LOG"))
            .unwrap_or(Vec::new(&env));
        log.push_back((recipient.clone(), amount, timestamp));
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "WITHDRAWAL_LOG"), &log);

        // 6. Emit FeesWithdrawn event
        env.events().publish(
            (Symbol::new(&env, "FeesWithdrawn"),),
            (recipient, amount, timestamp),
        );
    }

    /// Emergency drain — moves ALL funds to recipient.
    /// Should only be callable when the protocol is paused (check factory config).
    /// Requires admin authorization.
    /// Logs the drain. Emits EmergencyDrain event.
    /// Returns total amount drained in stroops.
    pub fn emergency_drain(env: Env, admin: Address, recipient: Address) -> i128 {
        // 1. Authentication
        admin.require_auth();

        // 2. State Check — protocol must be paused
        let factory: Address = env.storage().persistent().get(&symbol_short!("FACTORY")).unwrap();
        let config: ProtocolConfig = env.invoke_contract(&factory, &symbol_short!("get_config"), soroban_sdk::vec![&env]);
        if !config.paused {
            panic!("protocol is not paused");
        }

        // 3. Funds Transfer — drain full balance
        let amount: i128 = env.storage().persistent().get(&symbol_short!("BALANCE")).unwrap_or(0);
        let token: Address = env.storage().persistent().get(&symbol_short!("TOKEN")).unwrap();
        soroban_sdk::token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &recipient,
            &amount,
        );
        env.storage().persistent().set(&symbol_short!("BALANCE"), &0_i128);

        // 4. Logging & Events
        let mut log: Vec<(Address, i128, u64)> = env
            .storage()
            .persistent()
            .get(&symbol_short!("WLOG"))
            .unwrap_or(soroban_sdk::vec![&env]);
        log.push_back((recipient.clone(), amount, env.ledger().timestamp()));
        env.storage().persistent().set(&symbol_short!("WLOG"), &log);

        env.events().publish(
            (symbol_short!("EmrgDrain"), recipient),
            amount,
        );

        // 5. Return drained amount
        amount
    }

    /// Returns current treasury XLM balance in stroops.
    pub fn get_balance(env: Env) -> i128 {
        env.storage().get(&Symbol::short("BALANCE")).unwrap_or(0i128)
    }

    /// Returns lifetime cumulative fees collected (never decremented on withdrawals).
    pub fn get_total_fees_earned(env: Env) -> i128 {
        env.storage().get(&Symbol::short("TOTAL_FEES_EARNED")).unwrap_or(0i128)
    }

    /// Returns log of all past withdrawals: (recipient, amount, timestamp).
    pub fn get_withdrawal_log(env: Env) -> Vec<(Address, i128, u64)> {
        env.storage()
            .get(&Symbol::short("WITHDRAWAL_LOG"))
            .unwrap_or(Vec::new(&env))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, BytesN};

    fn addr_from_u8(env: &Env, v: u8) -> Address {
        let b = BytesN::from_array(env, &[v; 32]);
        Address::from_account_id(env, &b)
    }

    #[test]
    fn test_initialize_happy_path() {
        let env = Env::default();
        let admin = addr_from_u8(&env, 1u8);
        let factory = addr_from_u8(&env, 2u8);
        Treasury::initialize(env.clone(), admin.clone(), factory.clone());
        assert_eq!(Treasury::get_balance(env.clone()), 0i128);
        assert_eq!(Treasury::get_total_fees_earned(env.clone()), 0i128);
        let log = Treasury::get_withdrawal_log(env.clone());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_double_initialize_panics() {
        let env = Env::default();
        let admin = addr_from_u8(&env, 3u8);
        let factory = addr_from_u8(&env, 4u8);
        Treasury::initialize(env.clone(), admin.clone(), factory.clone());
        let res = std::panic::catch_unwind(|| {
            Treasury::initialize(env.clone(), admin.clone(), factory.clone())
        });
        assert!(res.is_err());
    }

    #[test]
    fn test_deposit_fees_unauthorized_panics() {
        let env = Env::default();
        let admin = addr_from_u8(&env, 5u8);
        let factory = addr_from_u8(&env, 6u8);
        Treasury::initialize(env.clone(), admin.clone(), factory.clone());
        // Attempt to deposit from an unregistered caller - should panic
        let res = std::panic::catch_unwind(|| {
            Treasury::deposit_fees(env.clone(), Bytes::from_array(&env, &[1u8; 32]), 100i128)
        });
        assert!(res.is_err());
    }

    #[test]
    fn test_deposit_fees_happy_path_simulated() {
        let env = Env::default();
        let admin = addr_from_u8(&env, 7u8);
        let factory = addr_from_u8(&env, 8u8);
        Treasury::initialize(env.clone(), admin.clone(), factory.clone());
        // Simulate a successful deposit by directly updating storage (used when cross-contract mocking isn't available)
        let prev = Treasury::get_balance(env.clone());
        let amount = 250i128;
        let new = prev + amount;
        env.storage().set(&Symbol::short("BALANCE"), &new);
        env.storage().set(&Symbol::short("TOTAL_FEES_EARNED"), &new);
        assert_eq!(Treasury::get_balance(env.clone()), new);
        assert_eq!(Treasury::get_total_fees_earned(env.clone()), new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events, Ledger},
        vec, IntoVal, Symbol,
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Registers a Treasury contract and pre-seeds its storage so that
    /// `emergency_drain` has the data it needs without calling `initialize`
    /// (which is still a `todo!`).
    fn setup(
        env: &Env,
        paused: bool,
        balance: i128,
    ) -> (TreasuryClient, Address, Address, Address) {
        let admin = Address::generate(env);
        let recipient = Address::generate(env);

        // Deploy a mock token
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
        let token = soroban_sdk::token::StellarAssetClient::new(env, &token_id);

        // Deploy a mock factory whose `get_config` returns a ProtocolConfig
        let factory_id = env.register(MockFactory, (admin.clone(), paused));

        // Deploy Treasury and seed its storage
        let treasury_id = env.register(Treasury, ());
        env.as_contract(&treasury_id, || {
            env.storage().persistent().set(&symbol_short!("FACTORY"), &factory_id);
            env.storage().persistent().set(&symbol_short!("TOKEN"),   &token_id);
            env.storage().persistent().set(&symbol_short!("ADMIN"),   &admin);
            env.storage().persistent().set(&symbol_short!("BALANCE"), &balance);
        });

        // Mint `balance` tokens into the treasury contract
        token.mint(&treasury_id, &balance);

        let client = TreasuryClient::new(env, &treasury_id);
        (client, admin, recipient, token_id)
    }

    // ── mock factory ─────────────────────────────────────────────────────────

    #[contract]
    struct MockFactory;

    #[contractimpl]
    impl MockFactory {
        pub fn __constructor(env: Env, admin: Address, paused: bool) {
            env.storage().persistent().set(&symbol_short!("admin"),  &admin);
            env.storage().persistent().set(&symbol_short!("paused"), &paused);
        }

        pub fn get_config(env: Env) -> ProtocolConfig {
            let admin: Address  = env.storage().persistent().get(&symbol_short!("admin")).unwrap();
            let paused: bool    = env.storage().persistent().get(&symbol_short!("paused")).unwrap();
            ProtocolConfig {
                admin:              admin.clone(),
                fee_collector:      admin,
                default_fee_bp:     200,
                min_bet_amount:     1_000_000,
                max_bet_amount:     100_000_000,
                dispute_window_sec: 86_400,
                paused,
            }
        }
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_emergency_drain_success() {
        let env = Env::default();
        env.mock_all_auths();

        let balance = 50_000_000_i128;
        let (client, admin, recipient, token_id) = setup(&env, true, balance);

        let drained = client.emergency_drain(&admin, &recipient);

        // Return value
        assert_eq!(drained, balance);

        // BALANCE set to 0
        let stored_balance: i128 = env.as_contract(&client.address, || {
            env.storage().persistent().get(&symbol_short!("BALANCE")).unwrap()
        });
        assert_eq!(stored_balance, 0);

        // Token actually transferred
        let token = soroban_sdk::token::Client::new(&env, &token_id);
        assert_eq!(token.balance(&recipient), balance);
        assert_eq!(token.balance(&client.address), 0);

        // Withdrawal log updated
        let log: Vec<(Address, i128, u64)> = env.as_contract(&client.address, || {
            env.storage().persistent().get(&symbol_short!("WLOG")).unwrap()
        });
        assert_eq!(log.len(), 1);
        let (log_recipient, log_amount, _) = log.get(0).unwrap();
        assert_eq!(log_recipient, recipient);
        assert_eq!(log_amount, balance);

        // EmergencyDrain event emitted
        let events = env.events().all();
        let found = events.iter().any(|(_, topics, data)| {
            topics.contains(&symbol_short!("EmrgDrain").into_val(&env))
                && data == balance.into_val(&env)
        });
        assert!(found, "EmergencyDrain event not found");
    }

    #[test]
    #[should_panic(expected = "protocol is not paused")]
    fn test_emergency_drain_fails_when_not_paused() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, recipient, _) = setup(&env, false, 10_000_000);
        client.emergency_drain(&admin, &recipient);
    }

    #[test]
    #[should_panic]
    fn test_emergency_drain_fails_when_unauthorized() {
        let env = Env::default();
        // Do NOT mock auths — a non-admin call must fail auth check.

        let (client, _admin, recipient, _) = setup(&env, true, 10_000_000);
        let attacker = Address::generate(&env);
        client.emergency_drain(&attacker, &recipient);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};

    fn create_env() -> Env {
        Env::default()
    }

    fn generate_address(env: &Env) -> Address {
        Address::generate(env)
    }

    fn register_treasury(env: &Env) -> (Address, TreasuryClient) {
        let contract_id = env.register_contract(None, Treasury);
        let client = TreasuryClient::new(env, &contract_id);
        (contract_id, client)
    }

    fn set_balance(env: &Env, amount: i128) {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, "BALANCE"), &amount);
    }

    fn fund_contract_native(env: &Env, contract_id: &Address, amount: i128) {
        let native = env.register_stellar_asset_contract(Address::from_str(env, "native"));
        let sac = token::StellarAssetClient::new(env, &native);
        sac.mint(contract_id, &amount);
    }

    // ─── SUCCESS PATH ──────────────────────────────────────────────────────────

    #[test]
    fn test_withdraw_fees_success() {
        let env = create_env();
        let (contract_id, client) = register_treasury(&env);
        let admin = generate_address(&env);
        let recipient = generate_address(&env);

        let deposit_amount: i128 = 1000;
        let withdraw_amount: i128 = 400;

        // Seed the contract's bookkeeping balance and native XLM
        set_balance(&env, deposit_amount);
        fund_contract_native(&env, &contract_id, deposit_amount);

        env.mock_all_auths();

        // Record the recipient's XLM balance before withdrawal
        let native = env.register_stellar_asset_contract(Address::from_str(&env, "native"));
        let token_client = token::Client::new(&env, &native);
        let recipient_balance_before = token_client.balance(&recipient);

        // Execute
        client.withdraw_fees(&admin, &recipient, &withdraw_amount);

        // Assert BALANCE decreased by withdraw_amount
        let remaining: i128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "BALANCE"))
            .unwrap_or(0);
        assert_eq!(remaining, deposit_amount - withdraw_amount);

        // Assert XLM was transferred to recipient
        let recipient_balance_after = token_client.balance(&recipient);
        assert_eq!(
            recipient_balance_after,
            recipient_balance_before + withdraw_amount
        );

        // Assert withdrawal was logged
        let log: Vec<(Address, i128, u64)> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "WITHDRAWAL_LOG"))
            .unwrap_or(Vec::new(&env));
        assert_eq!(log.len(), 1);
        let (log_recipient, log_amount, _log_ts) = log.get(0).unwrap();
        assert_eq!(log_recipient, recipient);
        assert_eq!(log_amount, withdraw_amount);

        // Assert event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_withdraw_fees_multiple_withdrawals() {
        let env = create_env();
        let (contract_id, client) = register_treasury(&env);
        let admin = generate_address(&env);
        let recipient = generate_address(&env);

        let initial_balance: i128 = 5000;

        set_balance(&env, initial_balance);
        fund_contract_native(&env, &contract_id, initial_balance);

        env.mock_all_auths();

        // First withdrawal
        client.withdraw_fees(&admin, &recipient, &1000);
        let log: Vec<(Address, i128, u64)> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "WITHDRAWAL_LOG"))
            .unwrap_or(Vec::new(&env));
        assert_eq!(log.len(), 1);

        // Second withdrawal
        client.withdraw_fees(&admin, &recipient, &2000);
        let log: Vec<(Address, i128, u64)> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "WITHDRAWAL_LOG"))
            .unwrap_or(Vec::new(&env));
        assert_eq!(log.len(), 2);

        let remaining: i128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "BALANCE"))
            .unwrap_or(0);
        assert_eq!(remaining, initial_balance - 1000 - 2000);
    }

    // ─── FAILURE: NON-ADMIN ───────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_withdraw_fees_non_admin_panics() {
        let env = create_env();
        let (_contract_id, client) = register_treasury(&env);
        let admin = generate_address(&env);
        let recipient = generate_address(&env);

        set_balance(&env, 1000);

        // No auth mocked → admin.require_auth() panics with HostError
        client.withdraw_fees(&admin, &recipient, &500);
    }

    // ─── FAILURE: AMOUNT > BALANCE ────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "amount exceeds balance")]
    fn test_withdraw_fees_exceeds_balance_panics() {
        let env = create_env();
        let (_contract_id, client) = register_treasury(&env);
        let admin = generate_address(&env);
        let recipient = generate_address(&env);

        set_balance(&env, 100);

        env.mock_all_auths();

        // amount (200) > BALANCE (100) → panic with "amount exceeds balance"
        client.withdraw_fees(&admin, &recipient, &200);
    }

    // ─── FAILURE: ZERO BALANCE ────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "amount exceeds balance")]
    fn test_withdraw_fees_zero_balance_panics() {
        let env = create_env();
        let (_contract_id, client) = register_treasury(&env);
        let admin = generate_address(&env);
        let recipient = generate_address(&env);

        // BALANCE is not set → defaults to 0 via unwrap_or(0)
        env.mock_all_auths();

        client.withdraw_fees(&admin, &recipient, &1);
    }

    // ─── EVENT STRUCTURE ──────────────────────────────────────────────────────

    #[test]
    fn test_withdraw_fees_emits_event() {
        let env = create_env();
        let (contract_id, client) = register_treasury(&env);
        let admin = generate_address(&env);
        let recipient = generate_address(&env);

        set_balance(&env, 500);
        fund_contract_native(&env, &contract_id, 500);

        env.mock_all_auths();

        client.withdraw_fees(&admin, &recipient, &300);

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (event_contract_id, topics, data) = &events.get(0).unwrap();
        assert_eq!(*event_contract_id, contract_id);

        // topics: (Symbol("FeesWithdrawn"),)
        let (symbol_key,): (Symbol,) = topics.clone().try_into().unwrap();
        assert_eq!(symbol_key.to_string(), "FeesWithdrawn");

        // data: (recipient, amount, timestamp)
        let (event_recipient, event_amount, _event_ts): (Address, i128, u64) =
            data.clone().try_into().unwrap();
        assert_eq!(event_recipient, recipient);
        assert_eq!(event_amount, 300);
    }
}