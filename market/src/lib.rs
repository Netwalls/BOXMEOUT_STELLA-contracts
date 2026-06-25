#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Vec, Symbol};
use crate::types::{Bet, BetSide, ClaimReceipt, Fighter, Market, MarketStatus, Outcome, WinningsClaimed, MarketResolved};

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
        // Emit resolution event before any status transition or early return.
        let market: Market = env
            .storage()
            .get(&Symbol::short("MARKET_INFO"))
            .expect("Market info not found");
        let resolved_at = env.ledger().timestamp();
        env.events().publish((Symbol::short("MarketResolved"),), MarketResolved {
            market_id: market.market_id.clone(),
            outcome: outcome.clone(),
            resolved_at,
        });

        // Minimal status update consistency for the resolved event.
        let mut updated_market = market;
        updated_market.status = if outcome == Outcome::NoContest {
            MarketStatus::Cancelled
        } else {
            MarketStatus::Resolved
        };
        env.storage().set(&Symbol::short("MARKET_INFO"), &updated_market);
    }

    /// Allows a winning bettor to claim proportional share of the pool.
    /// Validates: status==Resolved, bettor owns bet, side matches outcome, not already claimed.
    /// Payout = (bettor_stake / winning_pool) * total_pool * (1 - fee_bp/10000)
    /// Sends protocol fee to fee_collector.
    /// Marks bet as claimed. Emits WinningsClaimed event.
    /// Returns payout amount in stroops.
    pub fn claim_winnings(env: Env, bettor: Address, bet_id: Bytes) -> i128 {
        // Minimal implementation: emit WinningsClaimed event after a successful claim.
        // Full payout, fee calculations and transfers are expected in the complete implementation.
        let claimed_at: u64 = env.ledger().timestamp();
        let payout: i128 = 0;
        let fee_paid: i128 = 0;
        env.events().publish((Symbol::short("WinningsClaimed"),), WinningsClaimed {
            bet_id: bet_id.clone(),
            bettor: bettor.clone(),
            payout,
            fee_paid,
            claimed_at,
        });
        payout
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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, BytesN};

    fn addr_from_u8(env: &Env, v: u8) -> Address {
        let b = BytesN::from_array(env, &[v; 32]);
        Address::from_account_id(env, &b)
    }

    fn default_market(env: &Env, status: MarketStatus) -> Market {
        Market {
            market_id: Bytes::from_array(env, &[0u8; 32]),
            fighter_a: Fighter {
                name: "A".into_val(env),
                record: "0-0-0".into_val(env),
                nationality: "USA".into_val(env),
                weight_class: "Heavy".into_val(env),
            },
            fighter_b: Fighter {
                name: "B".into_val(env),
                record: "0-0-0".into_val(env),
                nationality: "BRA".into_val(env),
                weight_class: "Heavy".into_val(env),
            },
            scheduled_at: 1,
            betting_ends_at: 1,
            created_at: 1,
            created_by: addr_from_u8(env, 1),
            status,
            pool_a: 0,
            pool_b: 0,
            total_pool: 0,
            protocol_fee_bp: 100,
            oracle_address: addr_from_u8(env, 2),
        }
    }

    #[test]
    fn test_resolve_market_emits_event() {
        let env = Env::default();
        let market = default_market(&env, MarketStatus::Locked);
        env.storage().set(&Symbol::short("MARKET_INFO"), &market);

        let outcome = Outcome::FighterA;
        MarketContract::resolve_market(env.clone(), market.oracle_address.clone(), outcome.clone());

        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (topic, data_raw) = events[0].clone();
        let data: MarketResolved = data_raw.try_into().unwrap();
        assert_eq!(topic, Symbol::short("MarketResolved"));
        assert_eq!(data.market_id, market.market_id);
        assert_eq!(data.outcome, outcome);
        assert_eq!(data.resolved_at, env.ledger().timestamp());
    }

    #[test]
    fn test_resolve_market_emits_event_for_nocontest() {
        let env = Env::default();
        let market = default_market(&env, MarketStatus::Locked);
        env.storage().set(&Symbol::short("MARKET_INFO"), &market);

        let outcome = Outcome::NoContest;
        MarketContract::resolve_market(env.clone(), market.oracle_address.clone(), outcome.clone());

        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (topic, data_raw) = events[0].clone();
        let data: MarketResolved = data_raw.try_into().unwrap();
        assert_eq!(topic, Symbol::short("MarketResolved"));
        assert_eq!(data.market_id, market.market_id);
        assert_eq!(data.outcome, outcome);
        assert_eq!(data.resolved_at, env.ledger().timestamp());
    }
}
