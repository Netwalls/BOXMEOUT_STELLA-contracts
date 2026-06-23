#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Vec};
use crate::types::{Bet, BetSide, ClaimReceipt, Fighter, Market, MarketStatus, Outcome};

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// MARKET_INFO           -> Market
// BET_{bet_id}          -> Bet
// BETS_BY_ADDR_{addr}   -> Vec<Bytes>   (all bet_ids for an address)
// CLAIMED_{bet_id}      -> bool
// DISPUTE_RAISED        -> bool
// DISPUTE_REASON        -> String

#[contract]
pub struct MarketContract;

#[contractimpl]
impl MarketContract {

    /// Called by MarketFactory immediately after contract deployment.
    /// Stores all market metadata and initializes pool values to 0.
    /// Sets status to Open. Must only be callable by the factory address.
    pub fn initialize(
        env: Env,
        market_id: Bytes,
        fighter_a: Fighter,
        fighter_b: Fighter,
        scheduled_at: u64,
        betting_ends_at: u64,
        oracle: Address,
        factory: Address,
        protocol_fee_bp: u32,
        fee_collector: Address,
    ) {
        todo!("implement: verify caller == factory, build Market struct, store MARKET_INFO with status=Open")
    }

    /// Accepts XLM from bettor and records their bet in contract storage.
    /// Validates: market is Open, current time < betting_ends_at,
    /// amount within min/max bounds, bettor has authorized the call.
    /// Transfers XLM from bettor to this contract (escrow).
    /// Updates pool_a or pool_b. Generates unique bet_id.
    /// Emits BetPlaced event. Returns bet_id.
    pub fn place_bet(
        env: Env,
        bettor: Address,
        side: BetSide,
        amount: i128,
    ) -> Bytes {
        todo!("implement: require_auth(bettor), validate market/timing/amount, transfer XLM, update pool, generate bet_id, store Bet, emit event")
    }

    /// Transitions market status from Open to Locked.
    /// Callable by the oracle OR auto-triggered when betting_ends_at has passed.
    /// After locking, no new bets are accepted.
    /// Emits MarketLocked event.
    pub fn lock_market(env: Env, oracle: Address) {
        todo!("implement: verify caller==oracle OR ledger time > betting_ends_at, set status=Locked, emit event")
    }

    /// Called by oracle after fight concludes.
    /// Validates: caller == oracle, market status == Locked.
    /// Sets outcome and transitions status to Resolved.
    /// If outcome is NoContest, sets status to Cancelled for full refunds.
    /// Emits MarketResolved event.
    pub fn resolve_market(env: Env, oracle: Address, outcome: Outcome) {
        todo!("implement: require_auth(oracle), validate status==Locked, store outcome, set status=Resolved or Cancelled, emit event")
    }

    /// Allows a winning bettor to claim proportional share of the pool.
    /// Validates: status==Resolved, bettor owns bet, side matches outcome, not already claimed.
    /// Payout = (bettor_stake / winning_pool) * total_pool * (1 - fee_bp/10000)
    /// Sends protocol fee to fee_collector.
    /// Marks bet as claimed. Emits WinningsClaimed event.
    /// Returns payout amount in stroops.
    pub fn claim_winnings(env: Env, bettor: Address, bet_id: Bytes) -> i128 {
        todo!("implement: require_auth(bettor), validate eligibility, mark claimed BEFORE transfer (re-entrancy guard), compute payout, transfer XLM, emit event")
    }

    /// Issues a full refund for a bet when market is Cancelled or outcome is NoContest.
    /// No protocol fee deducted on refunds.
    /// Validates: status==Cancelled or outcome==NoContest, bettor owns bet, not claimed.
    /// Emits RefundClaimed event. Returns refund amount.
    pub fn claim_refund(env: Env, bettor: Address, bet_id: Bytes) -> i128 {
        todo!("implement: require_auth(bettor), validate market state, mark claimed BEFORE transfer, return full bet.amount, emit event")
    }

    /// Allows any bettor in this market to raise a dispute after resolution.
    /// Must be called within dispute_window_sec of resolved_at.
    /// Transitions status to Disputed — freezes all claim processing.
    /// Only one active dispute allowed per market.
    /// Emits DisputeRaised event.
    pub fn raise_dispute(env: Env, bettor: Address, reason: Bytes) {
        todo!("implement: require_auth(bettor), verify bettor has a bet on this market, check within window, check no existing dispute, set status=Disputed, store reason")
    }

    /// Admin-only. Settles a disputed market with a final override outcome.
    /// May differ from the oracle's original outcome.
    /// Transitions status back to Resolved. Claims re-open with new outcome.
    /// Emits DisputeResolved event.
    pub fn resolve_dispute(env: Env, admin: Address, override_outcome: Outcome) {
        todo!("implement: require_auth(admin), validate status==Disputed, update outcome, set status=Resolved, emit event")
    }

    /// Read-only. Returns the full Market struct.
    pub fn get_market_info(env: Env) -> Market {
        todo!("implement: read MARKET_INFO from storage and return")
    }

    /// Returns a specific Bet struct by its ID.
    /// Panics if bet_id is not found.
    pub fn get_bet(env: Env, bet_id: Bytes) -> Bet {
        todo!("implement: read BET_{{bet_id}} from storage, panic if missing")
    }

    /// Returns all bets placed by a specific address on this market.
    /// Returns empty Vec if address has no bets.
    pub fn get_bets_by_address(env: Env, bettor: Address) -> Vec<Bet> {
        todo!("implement: read BETS_BY_ADDR_{{bettor}} for bet_ids, map to Bet structs, return vec")
    }

    /// Read-only. Calculates the estimated payout for a given bet
    /// using current pool sizes. Does NOT modify state.
    /// Used by frontend to show live payout estimates before resolution.
    pub fn calculate_payout(env: Env, bet_id: Bytes) -> i128 {
        todo!("implement: read bet + market pools, apply payout formula, return estimated payout")
    }

    /// Read-only. Returns (pool_a, pool_b, implied_odds_a, implied_odds_b).
    /// implied_odds = pool_side / total_pool expressed as basis points (0-10000).
    /// Handles zero total_pool edge case (returns 5000/5000 even split).
    pub fn get_pool_odds(env: Env) -> (i128, i128, u32, u32) {
        todo!("implement: read pools from MARKET_INFO, compute implied odds, return tuple")
    }
}
