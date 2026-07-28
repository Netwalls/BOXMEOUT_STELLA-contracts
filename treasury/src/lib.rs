#![no_std]
use shared::types::ProtocolConfig;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Bytes, Env, Symbol, Vec,
};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// ADMIN              -> Address
// FACTORY            -> Address
// TOKEN              -> Address  (XLM token contract)
// BALANCE            -> i128
// TOTAL_FEES_EARNED  -> i128
// WITHDRAWAL_LOG     -> Vec<(Address, i128, u64)>
// ESCROW             -> Vec<(Bytes, i128)> mapping market_id to escrow balance
// FEE_COLLECTOR      -> Address

fn key_admin(env: &Env) -> Symbol {
    Symbol::new(env, "ADMIN")
}

fn key_factory(env: &Env) -> Symbol {
    Symbol::new(env, "FACTORY")
}

fn key_token(env: &Env) -> Symbol {
    Symbol::new(env, "TOKEN")
}

fn key_balance(env: &Env) -> Symbol {
    Symbol::new(env, "BALANCE")
}

fn key_total_fees(env: &Env) -> Symbol {
    Symbol::new(env, "TOTAL_FEES")
}

fn key_wlog(env: &Env) -> Symbol {
    Symbol::new(env, "WITHDRAWAL_LOG")
}

#[contract]
pub struct Treasury;

#[contractimpl]
impl Treasury {
    /// Sets up the Treasury with admin, authorized factory, and XLM token address.
    ///
    /// Must be called once immediately after deployment. Initializes `BALANCE` and
    /// `TOTAL_FEES_EARNED` to zero and sets up an empty `WITHDRAWAL_LOG`.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Address of the treasury administrator, authorized to withdraw funds.
    /// * `factory` - Address of the `MarketFactory` contract whose markets are permitted
    ///   to call deposit functions.
    /// * `token` - Address of the XLM token contract.
    ///
    /// # Panics
    ///
    /// Panics if the treasury has already been initialized.
    pub fn initialize(env: Env, admin: Address, factory: Address, token: Address) {
        if env.storage().persistent().has(&key_admin(&env)) {
            panic!("already initialized");
        }
        env.storage().persistent().set(&key_admin(&env), &admin);
        env.storage().persistent().set(&key_factory(&env), &factory);
        env.storage().persistent().set(&key_token(&env), &token);
        env.storage().persistent().set(&key_balance(&env), &0i128);
        env.storage()
            .persistent()
            .set(&key_total_fees(&env), &0i128);
        env.storage()
            .persistent()
            .set(&key_wlog(&env), &Vec::<(Address, i128, u64)>::new(&env));
    }

    /// Pull bet tokens into escrow, credited per market_id.
    ///
    /// Only callable by a Market contract address registered with the factory.
    /// Increments the per-market escrow balance and emits a `BetDeposited` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `from_market` - Address of the Market contract depositing bets (must be authorized).
    /// * `market_id` - Identifier of the market, used for per-market escrow tracking.
    /// * `amount` - Amount of XLM to deposit into escrow, in stroops.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The invoking contract is not a Market registered with the factory.
    /// - Token transfer fails.
    pub fn deposit(env: Env, from_market: Address, market_id: Bytes, amount: i128) {
        from_market.require_auth();

        let factory: Address = env
            .storage()
            .persistent()
            .get(&key_factory(&env))
            .expect("factory not set");

        let registered: Address = env.invoke_contract(
            &factory,
            &Symbol::new(&env, "get_market_address"),
            soroban_sdk::vec![&env, market_id.to_val()],
        );
        if registered != from_market {
            panic!("unauthorized: caller is not a registered market");
        }

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&key_token(&env))
            .expect("token not set");

        // Transfer XLM from market to treasury
        token::Client::new(&env, &token_addr).transfer(
            &from_market,
            &env.current_contract_address(),
            &amount,
        );

        // Increment per-market escrow using a map pattern with persistent storage
        // Store escrow as "ESCROW:<market_id_hex>" to keep it unique per market
        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows: Vec<(Bytes, i128)> = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .unwrap_or(Vec::new(&env));

        let mut found = false;
        for i in 0..escrows.len() {
            let (id, balance) = escrows.get(i).unwrap();
            if id == market_id {
                escrows.set(i, (id.clone(), balance + amount));
                found = true;
                break;
            }
        }
        if !found {
            escrows.push_back((market_id.clone(), amount));
        }

        env.storage()
            .persistent()
            .set(&escrow_key, &escrows);

        env.events().publish(
            (Symbol::new(&env, "BetDeposited"),),
            (from_market, &market_id, amount, env.ledger().timestamp()),
        );
    }

    /// Compute and route the protocol fee from a gross payout.
    ///
    /// Takes the gross payout amount, computes the fee based on the protocol fee rate,
    /// tracks the fee separately, and returns the net amount after fee deduction.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `gross_amount` - The total payout amount before fee deduction, in stroops.
    /// * `fee_bps` - Fee in basis points (e.g., 200 = 2%).
    ///
    /// # Returns
    ///
    /// Returns the net amount after fee deduction in stroops.
    pub fn collect_fee(env: Env, gross_amount: i128, fee_bps: u32) -> i128 {
        // Calculate fee using basis points formula: (amount * fee_bp) / 10_000
        let fee: i128 = gross_amount
            .checked_mul(fee_bps as i128)
            .expect("fee calculation overflow")
            .checked_div(10_000)
            .expect("fee calculation division error");

        // Track total fees earned
        let prev_fees: i128 = env
            .storage()
            .persistent()
            .get(&key_total_fees(&env))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&key_total_fees(&env), &(prev_fees + fee));

        // Emit fee collection event
        env.events().publish(
            (Symbol::new(&env, "FeeCollected"),),
            (gross_amount, fee_bps, fee, env.ledger().timestamp()),
        );

        // Return net amount after fee deduction
        gross_amount - fee
    }

    /// Pay a claimant out of a market's escrow, net of fee.
    ///
    /// Only callable by the originating Market contract. Calls `collect_fee` before
    /// transferring the net amount to the claimant. Decrements the per-market escrow.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `from_market` - Address of the Market contract releasing the winnings (must be authorized).
    /// * `market_id` - Identifier of the market from which to release escrow.
    /// * `claimant` - Address that will receive the winnings.
    /// * `gross_amount` - Gross payout amount before fee deduction, in stroops.
    /// * `fee_bps` - Fee in basis points (e.g., 200 = 2%).
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The caller is not authorized.
    /// - The market is not registered with the factory.
    /// - The escrow balance is insufficient.
    /// - Token transfer fails.
    pub fn release_winnings(
        env: Env,
        from_market: Address,
        market_id: Bytes,
        claimant: Address,
        gross_amount: i128,
        fee_bps: u32,
    ) {
        from_market.require_auth();

        let factory: Address = env
            .storage()
            .persistent()
            .get(&key_factory(&env))
            .expect("factory not set");

        let registered: Address = env.invoke_contract(
            &factory,
            &Symbol::new(&env, "get_market_address"),
            soroban_sdk::vec![&env, market_id.to_val()],
        );
        if registered != from_market {
            panic!("unauthorized: caller is not a registered market");
        }

        // Calculate net amount after fee
        let net_amount = Self::collect_fee(env.clone(), gross_amount, fee_bps);

        // Check and decrement per-market escrow
        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows: Vec<(Bytes, i128)> = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .unwrap_or(Vec::new(&env));

        let mut found = false;
        for i in 0..escrows.len() {
            let (id, balance) = escrows.get(i).unwrap();
            if id == market_id {
                if balance < gross_amount {
                    panic!("escrow balance insufficient");
                }
                escrows.set(i, (id.clone(), balance - gross_amount));
                found = true;
                break;
            }
        }
        if !found {
            panic!("no escrow for this market");
        }

        env.storage()
            .persistent()
            .set(&escrow_key, &escrows);

        // Transfer net amount to claimant
        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&key_token(&env))
            .expect("token not set");

        token::Client::new(&env, &token_addr).transfer(
            &env.current_contract_address(),
            &claimant,
            &net_amount,
        );

        env.events().publish(
            (Symbol::new(&env, "WinningsReleased"),),
            (from_market, &market_id, claimant, gross_amount, net_amount, env.ledger().timestamp()),
        );
    }

    /// Return full original stake with no fee, for cancelled markets.
    ///
    /// Only callable by a Market contract with Cancelled status. Returns the full
    /// original stake amount to the claimant without any fee deduction.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `from_market` - Address of the Market contract requesting the refund (must be authorized).
    /// * `market_id` - Identifier of the cancelled market.
    /// * `claimant` - Address that will receive the refund.
    /// * `refund_amount` - Full original stake amount, in stroops.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The caller is not authorized.
    /// - The market is not registered with the factory.
    /// - The escrow balance is insufficient.
    /// - Token transfer fails.
    pub fn refund(
        env: Env,
        from_market: Address,
        market_id: Bytes,
        claimant: Address,
        refund_amount: i128,
    ) {
        from_market.require_auth();

        let factory: Address = env
            .storage()
            .persistent()
            .get(&key_factory(&env))
            .expect("factory not set");

        let registered: Address = env.invoke_contract(
            &factory,
            &Symbol::new(&env, "get_market_address"),
            soroban_sdk::vec![&env, market_id.to_val()],
        );
        if registered != from_market {
            panic!("unauthorized: caller is not a registered market");
        }

        // Check and decrement per-market escrow
        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows: Vec<(Bytes, i128)> = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .unwrap_or(Vec::new(&env));

        let mut found = false;
        for i in 0..escrows.len() {
            let (id, balance) = escrows.get(i).unwrap();
            if id == market_id {
                if balance < refund_amount {
                    panic!("escrow balance insufficient");
                }
                escrows.set(i, (id.clone(), balance - refund_amount));
                found = true;
                break;
            }
        }
        if !found {
            panic!("no escrow for this market");
        }

        env.storage()
            .persistent()
            .set(&escrow_key, &escrows);

        // Transfer full amount to claimant (no fee deduction for refunds)
        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&key_token(&env))
            .expect("token not set");

        token::Client::new(&env, &token_addr).transfer(
            &env.current_contract_address(),
            &claimant,
            &refund_amount,
        );

        env.events().publish(
            (Symbol::new(&env, "RefundIssued"),),
            (from_market, &market_id, claimant, refund_amount, env.ledger().timestamp()),
        );
    }

    /// Receives protocol fees from a registered `Market` contract.
    ///
    /// Verifies the caller is the `Market` contract registered under `market_id`
    /// in the factory. Adds `amount` to both `BALANCE` and `TOTAL_FEES_EARNED`.
    /// Emits a `FeesDeposited` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `from_market` - Address of the Market contract depositing fees (must be authorized).
    /// * `market_id` - Identifier of the market depositing fees.
    /// * `amount` - Amount of XLM fees to deposit, in stroops.
    ///
    /// # Panics
    ///
    /// Panics if the invoking contract address does not match the address
    /// registered for `market_id` in the factory.
    pub fn deposit_fees(env: Env, from_market: Address, market_id: Bytes, amount: i128) {
        from_market.require_auth();

        let factory: Address = env
            .storage()
            .persistent()
            .get(&key_factory(&env))
            .expect("factory not set");

        let registered: Address = env.invoke_contract(
            &factory,
            &Symbol::new(&env, "get_market_address"),
            soroban_sdk::vec![&env, market_id.to_val()],
        );
        if registered != from_market {
            panic!("unauthorized: caller is not a registered market");
        }

        let balance: i128 = env
            .storage()
            .persistent()
            .get(&key_balance(&env))
            .unwrap_or(0);
        let total: i128 = env
            .storage()
            .persistent()
            .get(&key_total_fees(&env))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&key_balance(&env), &(balance + amount));
        env.storage()
            .persistent()
            .set(&key_total_fees(&env), &(total + amount));

        env.events().publish(
            (Symbol::new(&env, "FeesDeposited"),),
            (from_market, amount, env.ledger().timestamp()),
        );
    }

    /// Transfers collected fees to recipient. Only callable by admin.
    ///
    /// Validates that `amount ≤ BALANCE` and deducts it before transferring XLM.
    /// Appends an entry to `WITHDRAWAL_LOG`. Emits a `FeesWithdrawn` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Admin address. Must authorize this call.
    /// * `recipient` - Address that will receive the withdrawn XLM.
    /// * `amount` - Amount to withdraw in stroops. Must not exceed current `BALANCE`.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `admin` has not authorized the call.
    /// - `admin` is not the configured treasury admin.
    /// - `amount` exceeds the current `BALANCE`.
    pub fn withdraw_fees(env: Env, admin: Address, recipient: Address, amount: i128) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&key_admin(&env))
            .expect("not initialized");
        if stored_admin != admin {
            panic!("not admin");
        }

        let balance: i128 = env
            .storage()
            .persistent()
            .get(&key_balance(&env))
            .unwrap_or(0);
        if amount > balance {
            panic!("amount exceeds balance");
        }
        env.storage()
            .persistent()
            .set(&key_balance(&env), &(balance - amount));

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&key_token(&env))
            .expect("token not set");
        token::Client::new(&env, &token_addr).transfer(
            &env.current_contract_address(),
            &recipient,
            &amount,
        );

        let ts = env.ledger().timestamp();
        let mut log: Vec<(Address, i128, u64)> = env
            .storage()
            .persistent()
            .get(&key_wlog(&env))
            .unwrap_or(Vec::new(&env));
        log.push_back((recipient.clone(), amount, ts));
        env.storage().persistent().set(&key_wlog(&env), &log);

        env.events().publish(
            (Symbol::new(&env, "FeesWithdrawn"),),
            (recipient, amount, ts),
        );
    }

    /// Emergency drain — moves ALL funds to recipient.
    ///
    /// Only callable while the protocol is paused (verified via cross-contract call
    /// to the factory's `get_config`). Resets `BALANCE` to zero, logs the drain,
    /// and emits an `EmrgDrain` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Admin address. Must authorize this call and be the configured treasury admin.
    /// * `recipient` - Address that receives all drained XLM.
    ///
    /// # Returns
    ///
    /// Returns the total amount drained in stroops.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `admin` has not authorized the call.
    /// - `admin` is not the configured treasury admin.
    /// - The protocol is not currently paused.
    pub fn emergency_drain(env: Env, admin: Address, recipient: Address) -> i128 {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&key_admin(&env))
            .expect("not initialized");
        if stored_admin != admin {
            panic!("not admin");
        }

        let factory: Address = env
            .storage()
            .persistent()
            .get(&key_factory(&env))
            .expect("factory not set");
        let config: ProtocolConfig = env.invoke_contract(
            &factory,
            &Symbol::new(&env, "get_config"),
            soroban_sdk::vec![&env],
        );
        if !config.paused {
            panic!("protocol is not paused");
        }

        let amount: i128 = env
            .storage()
            .persistent()
            .get(&key_balance(&env))
            .unwrap_or(0);

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&key_token(&env))
            .expect("token not set");
        token::Client::new(&env, &token_addr).transfer(
            &env.current_contract_address(),
            &recipient,
            &amount,
        );
        env.storage()
            .persistent()
            .set(&key_balance(&env), &0i128);

        let ts = env.ledger().timestamp();
        let mut log: Vec<(Address, i128, u64)> = env
            .storage()
            .persistent()
            .get(&key_wlog(&env))
            .unwrap_or(Vec::new(&env));
        log.push_back((recipient.clone(), amount, ts));
        env.storage().persistent().set(&key_wlog(&env), &log);

        env.events().publish(
            (symbol_short!("EmrgDrain"), recipient.clone()),
            amount,
        );

        amount
    }

    /// Returns the current treasury XLM balance.
    ///
    /// Read-only — does not modify state.
    ///
    /// # Returns
    ///
    /// Returns the current `BALANCE` in stroops. Returns `0` if never set.
    pub fn get_balance(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&key_balance(&env))
            .unwrap_or(0)
    }

    /// Returns lifetime cumulative fees collected.
    ///
    /// This value is never decremented by withdrawals — it is a running total of
    /// all fees ever received. Read-only — does not modify state.
    ///
    /// # Returns
    ///
    /// Returns the cumulative `TOTAL_FEES_EARNED` in stroops. Returns `0` if never set.
    pub fn get_total_fees_earned(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&key_total_fees(&env))
            .unwrap_or(0)
    }

    /// Returns the complete log of all past withdrawals from the treasury.
    ///
    /// Each entry is a tuple of `(recipient, amount, timestamp)`. Read-only —
    /// does not modify state.
    ///
    /// # Returns
    ///
    /// Returns a [`Vec`] of `(Address, i128, u64)` tuples, one per withdrawal,
    /// in the order they occurred. Returns an empty `Vec` if no withdrawals have occurred.
    pub fn get_withdrawal_log(env: Env) -> Vec<(Address, i128, u64)> {
        env.storage()
            .persistent()
            .get(&key_wlog(&env))
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the current escrow balance for a specific market.
    ///
    /// Read-only — does not modify state.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `market_id` - The market identifier.
    ///
    /// # Returns
    ///
    /// Returns the escrow balance for the market in stroops, or `0` if no escrow exists.
    pub fn get_market_escrow(env: Env, market_id: Bytes) -> i128 {
        let escrow_key = Symbol::new(&env, "ESCROW");
        let escrows: Vec<(Bytes, i128)> = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .unwrap_or(Vec::new(&env));

        for i in 0..escrows.len() {
            let (id, balance) = escrows.get(i).unwrap();
            if id == market_id {
                return balance;
            }
        }
        0
    }
}

// ─── TESTS ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_core {
    use super::*;
    use shared::test_utils::{create_test_address, create_test_env, fund_address};
    use soroban_sdk::{contractimpl, token, Address as _, IntoVal};

    // ── Mock factory ──────────────────────────────────────────────────────────

    #[contract]
    struct MockFactory;

    #[contractimpl]
    impl MockFactory {
        pub fn get_config(env: Env) -> ProtocolConfig {
            let admin: Address = env
                .storage()
                .persistent()
                .get(&symbol_short!("admin"))
                .unwrap();
            let paused: bool = env
                .storage()
                .persistent()
                .get(&symbol_short!("paused"))
                .unwrap();
            ProtocolConfig {
                admin: admin.clone(),
                fee_collector: admin,
                default_fee_bp: 200,
                min_bet_amount: 1_000_000,
                max_bet_amount: 100_000_000,
                dispute_window_sec: 86_400,
                paused,
            }
        }

        pub fn get_market_address(env: Env, market_id: Bytes) -> Address {
            env.storage()
                .persistent()
                .get(&Symbol::new(&env, "MARKET"))
                .unwrap()
        }
    }

    fn setup(paused: bool, balance: i128) -> (Env, TreasuryClient, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        let sac = token::StellarAssetClient::new(&env, &token_id);

        let factory_id = env.register(MockFactory, ());
        env.as_contract(&factory_id, || {
            env.storage()
                .persistent()
                .set(&symbol_short!("admin"), &admin);
            env.storage()
                .persistent()
                .set(&symbol_short!("paused"), &paused);
        });

        let treasury_id = env.register(Treasury, ());
        let client = TreasuryClient::new(&env, &treasury_id);
        client.initialize(&admin, &factory_id, &token_id);

        if balance > 0 {
            sac.mint(&treasury_id, &balance);
            env.as_contract(&treasury_id, || {
                env.storage()
                    .persistent()
                    .set(&key_balance(&env), &balance);
            });
        }

        (env, client, admin, recipient, token_id)
    }

    // ── initialize ────────────────────────────────────────────────────────────

    #[test]
    fn test_initialize_happy_path() {
        let (env, client, _admin, _recipient, _token) = setup(false, 0);
        assert_eq!(client.get_balance(), 0);
        assert_eq!(client.get_total_fees_earned(), 0);
        assert_eq!(client.get_withdrawal_log().len(), 0);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize_panics() {
        let (env, client, admin, _r, token) = setup(false, 0);
        let factory = Address::generate(&env);
        client.initialize(&admin, &factory, &token);
    }

    // ── deposit ───────────────────────────────────────────────────────────────

    #[test]
    fn test_deposit_increments_escrow() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let market = Address::generate(&env);
        let market_id = Bytes::from_array(&env, &[1u8; 32]);

        let token_id = env
            .register_stellar_asset_contract_v2(Address::generate(&env))
            .address();
        let sac = token::StellarAssetClient::new(&env, &token_id);

        let factory_id = env.register(MockFactory, ());
        env.as_contract(&factory_id, || {
            env.storage()
                .persistent()
                .set(&symbol_short!("admin"), &admin);
            env.storage()
                .persistent()
                .set(&symbol_short!("paused"), &false);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "MARKET"), &market);
        });

        let treasury_id = env.register(Treasury, ());
        let client = TreasuryClient::new(&env, &treasury_id);
        client.initialize(&admin, &factory_id, &token_id);

        // Mint and deposit
        sac.mint(&market, &1000);
        client.deposit(&market, &market_id, &500i128);

        assert_eq!(client.get_market_escrow(&market_id), 500);
    }

    // ── collect_fee ───────────────────────────────────────────────────────────

    #[test]
    fn test_collect_fee_100_bp() {
        let (env, client, _admin, _recipient, _token) = setup(false, 0);
        let net = client.collect_fee(&1_000_000i128, &100u32);
        // 1% of 1_000_000 = 10_000 fee
        assert_eq!(net, 990_000);
    }

    #[test]
    fn test_collect_fee_200_bp() {
        let (env, client, _admin, _recipient, _token) = setup(false, 0);
        let net = client.collect_fee(&1_000_000i128, &200u32);
        // 2% of 1_000_000 = 20_000 fee
        assert_eq!(net, 980_000);
    }

    #[test]
    fn test_collect_fee_500_bp() {
        let (env, client, _admin, _recipient, _token) = setup(false, 0);
        let net = client.collect_fee(&1_000_000i128, &500u32);
        // 5% of 1_000_000 = 50_000 fee
        assert_eq!(net, 950_000);
    }

    // ── withdraw_fees ─────────────────────────────────────────────────────────

    #[test]
    fn test_withdraw_fees_happy_path() {
        let (env, client, admin, recipient, token_id) = setup(false, 1000);

        client.withdraw_fees(&admin, &recipient, &400i128);

        assert_eq!(client.get_balance(), 600);

        let tok = token::Client::new(&env, &token_id);
        assert_eq!(tok.balance(&recipient), 400);

        let log = client.get_withdrawal_log();
        assert_eq!(log.len(), 1);
        let (log_addr, log_amt, _) = log.get(0).unwrap();
        assert_eq!(log_addr, recipient);
        assert_eq!(log_amt, 400);
    }

    #[test]
    #[should_panic(expected = "amount exceeds balance")]
    fn test_withdraw_fees_over_withdraw_panics() {
        let (env, client, admin, recipient, _) = setup(false, 100);
        client.withdraw_fees(&admin, &recipient, &200i128);
    }

    // ── emergency_drain ───────────────────────────────────────────────────────

    #[test]
    fn test_emergency_drain_when_paused_succeeds() {
        let balance = 50_000_000i128;
        let (env, client, admin, recipient, token_id) = setup(true, balance);

        let drained = client.emergency_drain(&admin, &recipient);

        assert_eq!(drained, balance);
        assert_eq!(client.get_balance(), 0);

        let token = token::Client::new(&env, &token_id);
        assert_eq!(token.balance(&recipient), balance);
        assert_eq!(client.get_withdrawal_log().len(), 1);
    }

    #[test]
    #[should_panic(expected = "protocol is not paused")]
    fn test_emergency_drain_when_not_paused_panics() {
        let (env, client, admin, recipient, _) = setup(false, 10_000_000);
        client.emergency_drain(&admin, &recipient);
    }
}
