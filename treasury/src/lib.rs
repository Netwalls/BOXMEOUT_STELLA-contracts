#![no_std]
use shared::types::ProtocolConfig;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Bytes, Env, Symbol, Vec,
};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// "ADMIN"           -> Address
// "FACTORY"         -> Address
// "TOKEN"           -> Address  (XLM token contract)
// "BALANCE"         -> i128
// "TOTAL_FEES"      -> i128
// "FEE_BPS"         -> u32 (protocol fee rate in basis points)
// "WITHDRAWAL_LOG"  -> Vec<(Address, i128, u64)>

#[contracttype]
enum DataKey {
    Admin,
    Factory,
    Token,
    Balance,
    TotalFeesEarned,
    FeeBps,
    WithdrawalLog,
}

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

fn key_fee_bps(env: &Env) -> Symbol {
    Symbol::new(env, "FEE_BPS")
}

fn key_wlog(env: &Env) -> Symbol {
    Symbol::new(env, "WITHDRAWAL_LOG")
}

#[contract]
pub struct Treasury;

#[contractimpl]
impl Treasury {
    /// Sets up the Treasury with an admin and an authorized factory address.
    ///
    /// Must be called once immediately after deployment. Initializes `BALANCE` and
    /// `TOTAL_FEES_EARNED` to zero, sets `FEE_BPS` to default fee rate, and sets up
    /// an empty `WITHDRAWAL_LOG`.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Address of the treasury administrator, authorized to withdraw funds.
    /// * `factory` - Address of the `MarketFactory` contract whose markets are permitted
    ///   to call [`deposit_fees`].
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
        env.storage().persistent().set(&key_total_fees(&env), &0i128);
        env.storage().persistent().set(&key_fee_bps(&env), &0u32);
        env.storage()
            .persistent()
            .set(&key_wlog(&env), &Vec::<(Address, i128, u64)>::new(&env));
    }

    /// Sets the protocol fee rate in basis points.
    ///
    /// Only callable by the admin. Rejects values that exceed a reasonable ceiling (e.g., 10000 = 100%).
    /// Emits a `FeeBpsUpdated` event with the new fee rate.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Admin address. Must authorize this call.
    /// * `fee_bps` - New fee rate in basis points. Must not exceed 10000.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `admin` has not authorized the call.
    /// - `admin` is not the stored admin address.
    /// - `fee_bps` exceeds 10000 (the ceiling).
    pub fn set_fee_bps(env: Env, admin: Address, fee_bps: u32) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&key_admin(&env))
            .expect("not initialized");
        if stored_admin != admin {
            panic!("not admin");
        }

        if fee_bps > 10000 {
            panic!("fee_bps exceeds ceiling");
        }

        env.storage().persistent().set(&key_fee_bps(&env), &fee_bps);

        env.events().publish(
            (Symbol::new(&env, "FeeBpsUpdated"),),
            fee_bps,
        );
    }

    /// Receives protocol fees from a registered `Market` contract.
    ///
    /// Verifies the caller is the `Market` contract registered under `market_id`
    /// in the factory via a cross-contract call. Adds `amount` to both `BALANCE`
    /// and `TOTAL_FEES_EARNED`. Emits a `FeesDeposited` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `market_id` - Identifier of the market depositing fees, used to verify
    ///   the caller against the factory registry.
    /// * `amount` - Amount of XLM fees to deposit, in stroops.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The invoking contract address does not match the address registered for `market_id` in the factory.
    /// - The factory address has not been configured.
    pub fn deposit_fees(env: Env, market_id: Bytes, amount: i128) {
        let factory: Address = env
            .storage()
            .persistent()
            .get(&key_factory(&env))
            .expect("factory not set");

        let caller = env.current_contract_address();

        let registered: Address = env.invoke_contract(
            &factory,
            &Symbol::new(&env, "get_market_address"),
            soroban_sdk::vec![&env, market_id.to_val()],
        );
        if registered != caller {
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
            (caller, amount, env.ledger().timestamp()),
        );
    }

    /// Transfers collected fees to recipient. Only callable by admin or fee_recipient.
    ///
    /// Validates that `amount ≤ BALANCE` and deducts it before transferring XLM.
    /// Appends an entry to `WITHDRAWAL_LOG`. Emits a `FeesWithdrawn` event.
    /// Returns the amount withdrawn.
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
    /// - `amount` exceeds the current `BALANCE`.
    pub fn withdraw_fees(env: Env, admin: Address, recipient: Address, amount: i128) -> i128 {
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

        amount
    }

    /// Emergency drain — moves ALL funds to recipient.
    /// Only callable when protocol is paused. Requires admin authorization.
    ///
    /// Only callable while the protocol is paused (verified via cross-contract call
    /// to the factory's `get_config`). Resets `BALANCE` to zero, logs the drain,
    /// and emits an `EmergencyDrain` event.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    /// * `admin` - Admin address. Must authorize this call.
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
            (Symbol::new(&env, "EmergencyDrain"), recipient.clone()),
            amount,
        );

        amount
    }

    /// Returns the current treasury XLM balance.
    ///
    /// Read-only — does not modify state. Matches the sum of all deposits
    /// minus all withdrawals.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
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

    /// Returns the current protocol fee rate in basis points.
    ///
    /// Read-only — does not modify state.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
    ///
    /// # Returns
    ///
    /// Returns the current `FEE_BPS` value. Returns `0` if never set.
    pub fn get_fee_bps(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&key_fee_bps(&env))
            .unwrap_or(0)
    }

    /// Returns lifetime cumulative fees collected.
    ///
    /// This value is never decremented by withdrawals — it is a running total of
    /// all fees ever received. Read-only — does not modify state.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
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
    /// # Arguments
    ///
    /// * `env` - The Soroban execution environment.
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
}

// ─── TESTS ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use shared::test_utils::{create_test_address, create_test_env};
    use soroban_sdk::IntoVal;

    #[test]
    fn test_initialize() {
        let env = create_test_env();
        let admin = create_test_address(&env);
        let factory = create_test_address(&env);
        let token = create_test_address(&env);

        let contract_id = env.register_contract(None, Treasury);
        let client = TreasuryClient::new(&env, &contract_id);

        client.initialize(&admin, &factory, &token);

        assert_eq!(client.get_balance(), 0);
        assert_eq!(client.get_total_fees_earned(), 0);
        assert_eq!(client.get_fee_bps(), 0);
        assert_eq!(client.get_withdrawal_log().len(), 0);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize_panics() {
        let env = create_test_env();
        let admin = create_test_address(&env);
        let factory = create_test_address(&env);
        let token = create_test_address(&env);

        let contract_id = env.register_contract(None, Treasury);
        let client = TreasuryClient::new(&env, &contract_id);

        client.initialize(&admin, &factory, &token);
        client.initialize(&admin, &factory, &token);
    }

    #[test]
    fn test_set_fee_bps_success() {
        let env = create_test_env();
        env.mock_all_auths();

        let admin = create_test_address(&env);
        let factory = create_test_address(&env);
        let token = create_test_address(&env);

        let contract_id = env.register_contract(None, Treasury);
        let client = TreasuryClient::new(&env, &contract_id);

        client.initialize(&admin, &factory, &token);
        client.set_fee_bps(&admin, &200u32);

        assert_eq!(client.get_fee_bps(), 200);
    }

    #[test]
    #[should_panic(expected = "fee_bps exceeds ceiling")]
    fn test_set_fee_bps_exceeds_ceiling() {
        let env = create_test_env();
        env.mock_all_auths();

        let admin = create_test_address(&env);
        let factory = create_test_address(&env);
        let token = create_test_address(&env);

        let contract_id = env.register_contract(None, Treasury);
        let client = TreasuryClient::new(&env, &contract_id);

        client.initialize(&admin, &factory, &token);
        client.set_fee_bps(&admin, &10001u32);
    }

    #[test]
    #[should_panic(expected = "not admin")]
    fn test_set_fee_bps_not_admin_panics() {
        let env = create_test_env();
        env.mock_all_auths();

        let admin = create_test_address(&env);
        let factory = create_test_address(&env);
        let token = create_test_address(&env);
        let attacker = create_test_address(&env);

        let contract_id = env.register_contract(None, Treasury);
        let client = TreasuryClient::new(&env, &contract_id);

        client.initialize(&admin, &factory, &token);
        client.set_fee_bps(&attacker, &200u32);
    }

    #[test]
    fn test_get_balance_empty() {
        let env = create_test_env();
        let admin = create_test_address(&env);
        let factory = create_test_address(&env);
        let token = create_test_address(&env);

        let contract_id = env.register_contract(None, Treasury);
        let client = TreasuryClient::new(&env, &contract_id);

        client.initialize(&admin, &factory, &token);
        assert_eq!(client.get_balance(), 0);
    }
}
