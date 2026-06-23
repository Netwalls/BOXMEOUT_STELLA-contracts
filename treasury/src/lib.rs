#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, Vec};

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
        todo!("implement: panic if already initialized, store ADMIN + FACTORY, set BALANCE=0, TOTAL_FEES_EARNED=0")
    }

    /// Called by Market contracts when distributing protocol fees on claim.
    /// Validates caller is a Market contract registered in the factory.
    /// Adds amount to BALANCE and TOTAL_FEES_EARNED.
    /// Emits FeesDeposited event.
    pub fn deposit_fees(env: Env, market_id: Bytes, amount: i128) {
        todo!("implement: verify caller is a registered market contract via factory, update BALANCE and TOTAL_FEES_EARNED, emit event")
    }

    /// Transfers collected fees to a recipient (e.g. DAO multisig, team wallet).
    /// Validates: caller is admin, amount <= BALANCE.
    /// Appends withdrawal to WITHDRAWAL_LOG.
    /// Emits FeesWithdrawn event.
    pub fn withdraw_fees(env: Env, admin: Address, recipient: Address, amount: i128) {
        todo!("implement: require_auth(admin), check amount <= BALANCE, deduct BALANCE, transfer XLM to recipient, log withdrawal, emit event")
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
        todo!("implement: read BALANCE from storage and return")
    }

    /// Returns lifetime cumulative fees collected (never decremented on withdrawals).
    pub fn get_total_fees_earned(env: Env) -> i128 {
        todo!("implement: read TOTAL_FEES_EARNED from storage and return")
    }

    /// Returns log of all past withdrawals: (recipient, amount, timestamp).
    pub fn get_withdrawal_log(env: Env) -> Vec<(Address, i128, u64)> {
        todo!("implement: read WITHDRAWAL_LOG from storage and return")
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
