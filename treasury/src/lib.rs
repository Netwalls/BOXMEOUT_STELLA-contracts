#![no_std]
/// ============================================================
/// BOXMEOUT — Treasury Contract

use soroban_sdk::{contract, contractimpl, Address, Env, Map};

use boxmeout_shared::errors::ContractError;

// ─── Storage Key Constants ────────────────────────────────────────────────────

/// Address — treasury admin (multisig recommended in production)
const ADMIN: &str = "ADMIN";
/// Map<Address, i128> — token contract address → total fees accumulated
const ACCUMULATED_FEES: &str = "ACCUMULATED_FEES";
/// Vec<Address> — market contracts permitted to call deposit_fees()
const APPROVED_MARKETS: &str = "APPROVED_MARKETS";
/// i128 — maximum stroops withdrawable in a single transaction
const WITHDRAWAL_LIMIT: &str = "WITHDRAWAL_LIMIT";
/// Map<u64, i128> — unix_day_bucket → stroops withdrawn that day
/// day_bucket = floor(timestamp / 86400)
const DAILY_WITHDRAWN: &str = "DAILY_WITHDRAWN";

#[contract]
pub struct Treasury;

#[contractimpl]
impl Treasury {
    /// Initializes the treasury.
    /// Stores admin, sets withdrawal_limit, initializes empty fee map.
    /// Returns ContractError::AlreadyInitialized on second call.
    pub fn initialize(
        env: Env,
        admin: Address,
        withdrawal_limit: i128,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Registers a Market contract as approved to deposit fees.
    /// Only callable by admin.
    /// Idempotent — approving an already-approved address is a no-op.
    pub fn approve_market(
        env: Env,
        admin: Address,
        market_address: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Removes a market from the approved list.
    /// Only callable by admin.
    /// Does not affect fees already deposited by this market.
    pub fn revoke_market(
        env: Env,
        admin: Address,
        market_address: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Called by an approved Market contract to deposit collected fees.
    ///
    /// Validation:
    ///   1. Verify caller (market) is in APPROVED_MARKETS
    ///   2. Transfer `amount` of `token` from caller to this contract
    ///   3. Increment ACCUMULATED_FEES[token] by amount
    ///   4. Emit FeeDeposited event
    ///
    /// Returns ContractError::MarketNotApproved if caller is not registered.
    pub fn deposit_fees(
        env: Env,
        market: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Withdraws accumulated fees to destination address.
    ///
    /// Validation:
    ///   1. Require admin authorization
    ///   2. amount <= WITHDRAWAL_LIMIT (per-transaction cap)
    ///   3. Daily total (DAILY_WITHDRAWN[today] + amount) <= WITHDRAWAL_LIMIT * 5
    ///      (daily cap = 5x single-transaction limit; adjust as needed)
    ///   4. ACCUMULATED_FEES[token] >= amount
    ///
    /// Side effects:
    ///   - Decrement ACCUMULATED_FEES[token]
    ///   - Increment DAILY_WITHDRAWN[today]
    ///   - Transfer token to destination
    ///   - Emit FeeWithdrawn event
    pub fn withdraw_fees(
        env: Env,
        admin: Address,
        token: Address,
        amount: i128,
        destination: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Returns total accumulated fees for a given token in stroops.
    pub fn get_accumulated_fees(env: Env, token: Address) -> i128 {
        todo!()
    }

    /// Returns the total stroops withdrawn today (rolling calendar day, UTC).
    /// day_bucket = floor(env.ledger().timestamp() / 86400)
    pub fn get_daily_withdrawal_amount(env: Env) -> i128 {
        todo!()
    }

    /// Updates the maximum amount withdrawable in a single transaction.
    /// Requires admin authorization.
    pub fn update_withdrawal_limit(
        env: Env,
        admin: Address,
        new_limit: i128,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Emergency function — transfers all of `token` held by this contract to admin.
    ///
    /// Use only when the contract is suspected to be compromised.
    /// Zeroes ACCUMULATED_FEES[token].
    /// Emits EmergencyDrain event.
    /// Requires admin authorization.
    pub fn emergency_drain(
        env: Env,
        admin: Address,
        token: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }
}
