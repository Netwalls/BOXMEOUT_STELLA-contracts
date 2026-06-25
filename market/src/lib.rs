#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Vec, Symbol};
use crate::types::{Bet, BetSide, ClaimReceipt, Fighter, Market, MarketStatus, Outcome, WinningsClaimed, MarketResolved};
use crate::types::{Bet, BetSide, ClaimReceipt, Fighter, Market, MarketStatus, Outcome};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, String, Vec};

const MARKET_INFO_KEY: &str = "market_info";
const NEXT_BET_ID_KEY: &str = "next_bet_id";

// ─── STORAGE KEYS ─────────────────────────────────────────────────────────────
// MARKET_INFO           -> Market
// BET_{bet_id}          -> Bet
// BETS_BY_ADDR_{addr}   -> Vec<Bytes>   (all bet_ids for an address)
// CLAIMED_{bet_id}      -> bool
// DISPUTE_RAISED        -> bool
// DISPUTE_REASON        -> String

#[contract]
pub struct MarketContract;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetPlacedEvent {
    pub bet_id: Bytes,
    pub market_id: Bytes,
    pub bettor: Address,
    pub side: BetSide,
    pub amount: i128,
    pub placed_at: u64,
}

#[contractimpl]
impl MarketContract {
    fn read_market(env: &Env) -> Market {
        env.storage().persistent().get(&MARKET_INFO_KEY).unwrap().unwrap()
    }

    fn write_market(env: &Env, market: &Market) {
        env.storage().persistent().set(&MARKET_INFO_KEY, market);
    }

    fn read_next_bet_id(env: &Env) -> u64 {
        env.storage().persistent().get(&NEXT_BET_ID_KEY).unwrap_or(1u64)
    }

    fn write_next_bet_id(env: &Env, id: u64) {
        env.storage().persistent().set(&NEXT_BET_ID_KEY, &id);
    }

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
        let _ = (factory, fee_collector, protocol_fee_bp);
        let market = Market {
            market_id: market_id.clone(),
            fighter_a,
            fighter_b,
            scheduled_at,
            betting_ends_at,
            created_at: env.ledger().timestamp(),
            created_by: env.current_contract_address(),
            status: MarketStatus::Open,
            pool_a: 0,
            pool_b: 0,
            total_pool: 0,
            protocol_fee_bp,
            oracle_address: oracle,
        };
        env.storage().persistent().set(&MARKET_INFO_KEY, &market);
        env.storage().persistent().set(&NEXT_BET_ID_KEY, &1u64);
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
        bettor.require_auth();

        let mut market = Self::read_market(&env);
        assert!(matches!(market.status, MarketStatus::Open));
        assert!(env.ledger().timestamp() < market.betting_ends_at);
        assert!(amount > 0);

        if matches!(side, BetSide::FighterA) {
            market.pool_a += amount;
        } else {
            market.pool_b += amount;
        }
        market.total_pool += amount;

        let next_bet_id = Self::read_next_bet_id(&env);
        let mut bet_id_bytes = [0u8; 32];
        bet_id_bytes[..8].copy_from_slice(&next_bet_id.to_be_bytes());
        let bet_id = Bytes::from_array(&bet_id_bytes);

        let bet = Bet {
            bet_id: bet_id.clone(),
            market_id: market.market_id.clone(),
            bettor: bettor.clone(),
            side: side.clone(),
            amount,
            placed_at: env.ledger().timestamp(),
            claimed: false,
        };
        env.storage().persistent().set(&bet_id, &bet);
        Self::write_market(&env, &market);
        Self::write_next_bet_id(&env, next_bet_id + 1);

        let event = BetPlacedEvent {
            bet_id: bet_id.clone(),
            market_id: market.market_id.clone(),
            bettor: bettor.clone(),
            side: side.clone(),
            amount,
            placed_at: env.ledger().timestamp(),
        };
        env.events().publish((symbol_short!("bet_placed"),), event);

        bet_id
    }

    /// Transitions market status from Open to Locked.
    /// Callable by the oracle OR auto-triggered when betting_ends_at has passed.
    /// After locking, no new bets are accepted.
    /// Emits MarketLocked event.
    pub fn lock_market(env: Env, oracle: Address) {
        let _ = (env, oracle);
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
        let _ = (env, oracle, outcome);
        todo!("implement: require_auth(oracle), validate status==Locked, store outcome, set status=Resolved or Cancelled, emit event")
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
        let _ = (env, bettor, bet_id);
        todo!("implement: require_auth(bettor), validate eligibility, mark claimed BEFORE transfer (re-entrancy guard), compute payout, transfer XLM, emit event")
    }

    /// Issues a full refund for a bet when market is Cancelled or outcome is NoContest.
    /// No protocol fee deducted on refunds.
    /// Validates: status==Cancelled or outcome==NoContest, bettor owns bet, not claimed.
    /// Emits RefundClaimed event. Returns refund amount.
    pub fn claim_refund(env: Env, bettor: Address, bet_id: Bytes) -> i128 {
        let _ = (env, bettor, bet_id);
        todo!("implement: require_auth(bettor), validate market state, mark claimed BEFORE transfer, return full bet.amount, emit event")
    }

    /// Allows any bettor in this market to raise a dispute after resolution.
    /// Must be called within dispute_window_sec of resolved_at.
    /// Transitions status to Disputed — freezes all claim processing.
    /// Only one active dispute allowed per market.
    /// Emits DisputeRaised event.
    pub fn raise_dispute(env: Env, bettor: Address, reason: Bytes) {
        let _ = (env, bettor, reason);
        todo!("implement: require_auth(bettor), verify bettor has a bet on this market, check within window, check no existing dispute, set status=Disputed, store reason")
    }

    /// Admin-only. Settles a disputed market with a final override outcome.
    /// May differ from the oracle's original outcome.
    /// Transitions status back to Resolved. Claims re-open with new outcome.
    /// Emits DisputeResolved event.
    pub fn resolve_dispute(env: Env, admin: Address, override_outcome: Outcome) {
        let _ = (env, admin, override_outcome);
        todo!("implement: require_auth(admin), validate status==Disputed, update outcome, set status=Resolved, emit event")
    }

    /// Read-only. Returns the full Market struct.
    pub fn get_market_info(env: Env) -> Market {
        let _ = env;
        todo!("implement: read MARKET_INFO from storage and return")
    }

    /// Returns a specific Bet struct by its ID.
    /// Panics if bet_id is not found.
    pub fn get_bet(env: Env, bet_id: Bytes) -> Bet {
        let _ = (env, bet_id);
        todo!("implement: read BET_{{bet_id}} from storage, panic if missing")
    }

    /// Returns all bets placed by a specific address on this market.
    /// Returns empty Vec if address has no bets.
    pub fn get_bets_by_address(env: Env, bettor: Address) -> Vec<Bet> {
        let _ = (env, bettor);
        todo!("implement: read BETS_BY_ADDR_{{bettor}} for bet_ids, map to Bet structs, return vec")
    }

    /// Read-only. Calculates the estimated payout for a given bet
    /// using current pool sizes. Does NOT modify state.
    /// Used by frontend to show live payout estimates before resolution.
    pub fn calculate_payout(env: Env, bet_id: Bytes) -> i128 {
        let _ = (env, bet_id);
        todo!("implement: read bet + market pools, apply payout formula, return estimated payout")
    }

    /// Read-only. Returns (pool_a, pool_b, implied_odds_a, implied_odds_b).
    /// implied_odds = pool_side / total_pool expressed as basis points (0-10000).
    /// Handles zero total_pool edge case (returns 5000/5000 even split).
    pub fn get_pool_odds(env: Env) -> (i128, i128, u32, u32) {
        let _ = env;
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
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn place_bet_emits_bet_placed_event() {
        let env = Env::default();
        let contract_id = env.register_contract(None, MarketContract);
        let client = MarketContractClient::new(&env, &contract_id);

        let bettor = Address::generate(&env);
        let oracle = Address::generate(&env);
        let fighter_a = Fighter {
            name: String::from_str(&env, "A"),
            record: String::from_str(&env, "10-0"),
            nationality: String::from_str(&env, "US"),
            weight_class: String::from_str(&env, "Heavyweight"),
        };
        let fighter_b = Fighter {
            name: String::from_str(&env, "B"),
            record: String::from_str(&env, "9-1"),
            nationality: String::from_str(&env, "MX"),
            weight_class: String::from_str(&env, "Heavyweight"),
        };
        let market_id = Bytes::from_array(&[1u8; 32]);
        client.initialize(
            &market_id,
            &fighter_a,
            &fighter_b,
            &100u64,
            &200u64,
            &oracle,
            &Address::generate(&env),
            &0u32,
            &Address::generate(&env),
        );

        let bet_id = client.place_bet(&bettor, &BetSide::FighterA, &100i128);
        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let event = events.get(0).unwrap().unwrap();
        let topics = event.0;
        assert_eq!(topics.len(), 1);
        assert_eq!(topics.get(0).unwrap(), symbol_short!("bet_placed"));

        let data = event.1;
        assert_eq!(
            data,
            BetPlacedEvent {
                bet_id: bet_id.clone(),
                market_id: market_id.clone(),
                bettor: bettor.clone(),
                side: BetSide::FighterA,
                amount: 100i128,
                placed_at: env.ledger().timestamp(),
            }
        );
    }
}
