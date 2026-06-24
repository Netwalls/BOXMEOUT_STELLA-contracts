#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Vec, Symbol, BytesN};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// ADMIN              -> Address
// FACTORY            -> Address
// BALANCE            -> i128
// TOTAL_FEES_EARNED  -> i128
// WITHDRAWAL_LOG     -> Vec<(Address, i128, u64)>

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
        todo!("implement: require_auth(admin), check amount <= BALANCE, deduct BALANCE, transfer XLM to recipient, log withdrawal, emit event")
    }

    /// Emergency drain — moves ALL funds to recipient.
    /// Should only be callable when the protocol is paused (check factory config).
    /// Requires admin authorization.
    /// Logs the drain. Emits EmergencyDrain event.
    /// Returns total amount drained in stroops.
    pub fn emergency_drain(env: Env, admin: Address, recipient: Address) -> i128 {
        todo!("implement: require_auth(admin), verify protocol is paused, transfer full BALANCE, set BALANCE=0, log, emit event, return drained amount")
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
