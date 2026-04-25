/// ============================================================
/// BOXMEOUT — Contract Events
/// All emitted events are defined here for consistency.
/// ============================================================

use soroban_sdk::{Address, Env, String, Symbol, Vec};

use crate::types::{BetRecord, ClaimReceipt, Outcome};

pub fn emit_market_created(env: &Env, market_id: u64, contract_address: Address, match_id: String) {
    let topics = (Symbol::new(env, "market_created"), market_id);
    env.events().publish(topics, (contract_address, match_id));
}

pub fn emit_market_locked(env: &Env, market_id: u64) {
    let topics = (Symbol::new(env, "market_locked"), market_id);
    env.events().publish(topics, ());
}

pub fn emit_market_resolved(env: &Env, market_id: u64, outcome: Outcome, oracle_address: Address) {
    let topics = (Symbol::new(env, "market_resolved"), market_id);
    env.events().publish(topics, (outcome, oracle_address));
}

pub fn emit_bet_placed(env: &Env, market_id: u64, bet: BetRecord) {
    let topics = (Symbol::new(env, "bet_placed"), market_id);
    env.events().publish(topics, bet);
}

pub fn emit_winnings_claimed(env: &Env, market_id: u64, receipt: ClaimReceipt) {
    let topics = (Symbol::new(env, "winnings_claimed"), market_id);
    env.events().publish(topics, receipt);
}

pub fn emit_refund_claimed(env: &Env, market_id: u64, bettor: Address, amount: i128) {
    let topics = (Symbol::new(env, "refund_claimed"), market_id);
    env.events().publish(topics, (bettor, amount));
}

pub fn emit_market_cancelled(env: &Env, market_id: u64, reason: String) {
    let topics = (Symbol::new(env, "market_cancelled"), market_id);
    env.events().publish(topics, reason);
}

pub fn emit_market_disputed(env: &Env, market_id: u64, reason: String) {
    let topics = (Symbol::new(env, "market_disputed"), market_id);
    env.events().publish(topics, reason);
}

pub fn emit_dispute_resolved(env: &Env, market_id: u64, final_outcome: Outcome) {
    let topics = (Symbol::new(env, "dispute_resolved"), market_id);
    env.events().publish(topics, final_outcome);
}

pub fn emit_admin_transferred(env: &Env, old_admin: Address, new_admin: Address) {
    let topics = (Symbol::new(env, "admin_transferred"),);
    env.events().publish(topics, (old_admin, new_admin));
}

pub fn emit_fee_deposited(env: &Env, market: Address, token: Address, amount: i128) {
    let topics = (Symbol::new(env, "fee_deposited"),);
    env.events().publish(topics, (market, token, amount));
}

pub fn emit_fee_withdrawn(env: &Env, token: Address, amount: i128, destination: Address) {
    let topics = (Symbol::new(env, "fee_withdrawn"),);
    env.events().publish(topics, (token, amount, destination));
}

pub fn emit_emergency_drain(env: &Env, token: Address, amount: i128) {
    let topics = (Symbol::new(env, "emergency_drain"),);
    env.events().publish(topics, (token, amount));
}

pub fn emit_config_updated(env: &Env, param_name: String, new_value: i128) {
    let topics = (Symbol::new(env, "config_updated"),);
    env.events().publish(topics, (param_name, new_value));
}

pub fn emit_conflicting_oracle_report(env: &Env, market_id: u64, oracle_address: Address) {
    let topics = (Symbol::new(env, "conflicting_oracle_report"), market_id);
    env.events().publish(topics, oracle_address);
}

#[cfg(test)]
mod tests {
    use soroban_sdk::{
        testutils::{Address as _, Events},
        Address, Env, IntoVal, Symbol,
    };

    use crate::{
        events::*,
        types::{BetRecord, BetSide, ClaimReceipt, Outcome},
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    fn env() -> Env {
        Env::default()
    }

    fn addr(env: &Env) -> Address {
        Address::generate(env)
    }

    fn str(env: &Env, s: &str) -> soroban_sdk::String {
        soroban_sdk::String::from_slice(env, s)
    }

    // Grab the single event emitted and return (topics_val, data_val).
    // Panics if != 1 event was emitted.
    macro_rules! sole_event {
        ($env:expr) => {{
            let all = $env.events().all();
            assert_eq!(all.len(), 1, "expected exactly 1 event");
            let ev = all.get(0).unwrap();
            (ev.1, ev.2) // (topics, data)
        }};
    }

    // ── market_created ───────────────────────────────────────────────────────

    #[test]
    fn test_emit_market_created() {
        let env = env();
        let contract = addr(&env);
        emit_market_created(&env, 1, contract.clone(), str(&env, "FURY-USYK-2025"));

        let (topics, data) = sole_event!(env);
        assert_eq!(
            topics,
            (Symbol::new(&env, "market_created"), 1_u64).into_val(&env)
        );
        assert_eq!(
            data,
            (contract, str(&env, "FURY-USYK-2025")).into_val(&env)
        );
    }

    // ── market_locked ────────────────────────────────────────────────────────

    #[test]
    fn test_emit_market_locked() {
        let env = env();
        emit_market_locked(&env, 2);

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "market_locked"), 2_u64).into_val(&env));
        assert_eq!(data, ().into_val(&env));
    }

    // ── market_resolved ──────────────────────────────────────────────────────

    #[test]
    fn test_emit_market_resolved() {
        let env = env();
        let oracle = addr(&env);
        emit_market_resolved(&env, 3, Outcome::FighterA, oracle.clone());

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "market_resolved"), 3_u64).into_val(&env));
        assert_eq!(data, (Outcome::FighterA, oracle).into_val(&env));
    }

    // ── bet_placed ───────────────────────────────────────────────────────────

    #[test]
    fn test_emit_bet_placed() {
        let env = env();
        let bettor = addr(&env);
        let bet = BetRecord {
            bettor: bettor.clone(),
            market_id: 4,
            side: BetSide::FighterA,
            amount: 5_000_000,
            placed_at: 1_000_000,
            claimed: false,
        };
        emit_bet_placed(&env, 4, bet.clone());

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "bet_placed"), 4_u64).into_val(&env));
        assert_eq!(data, bet.into_val(&env));
    }

    // ── winnings_claimed ─────────────────────────────────────────────────────

    #[test]
    fn test_emit_winnings_claimed() {
        let env = env();
        let bettor = addr(&env);
        let receipt = ClaimReceipt {
            bettor: bettor.clone(),
            market_id: 5,
            amount_won: 9_800_000,
            fee_deducted: 200_000,
            claimed_at: 2_000_000,
        };
        emit_winnings_claimed(&env, 5, receipt.clone());

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "winnings_claimed"), 5_u64).into_val(&env));
        assert_eq!(data, receipt.into_val(&env));
    }

    // ── refund_claimed ───────────────────────────────────────────────────────

    #[test]
    fn test_emit_refund_claimed() {
        let env = env();
        let bettor = addr(&env);
        emit_refund_claimed(&env, 6, bettor.clone(), 5_000_000);

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "refund_claimed"), 6_u64).into_val(&env));
        assert_eq!(data, (bettor, 5_000_000_i128).into_val(&env));
    }

    // ── market_cancelled ─────────────────────────────────────────────────────

    #[test]
    fn test_emit_market_cancelled() {
        let env = env();
        emit_market_cancelled(&env, 7, str(&env, "fight_postponed"));

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "market_cancelled"), 7_u64).into_val(&env));
        assert_eq!(data, str(&env, "fight_postponed").into_val(&env));
    }

    // ── market_disputed ──────────────────────────────────────────────────────

    #[test]
    fn test_emit_market_disputed() {
        let env = env();
        emit_market_disputed(&env, 8, str(&env, "oracle_conflict"));

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "market_disputed"), 8_u64).into_val(&env));
        assert_eq!(data, str(&env, "oracle_conflict").into_val(&env));
    }

    // ── dispute_resolved ─────────────────────────────────────────────────────

    #[test]
    fn test_emit_dispute_resolved() {
        let env = env();
        emit_dispute_resolved(&env, 9, Outcome::Draw);

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "dispute_resolved"), 9_u64).into_val(&env));
        assert_eq!(data, Outcome::Draw.into_val(&env));
    }

    // ── admin_transferred ────────────────────────────────────────────────────

    #[test]
    fn test_emit_admin_transferred() {
        let env = env();
        let old = addr(&env);
        let new = addr(&env);
        emit_admin_transferred(&env, old.clone(), new.clone());

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "admin_transferred"),).into_val(&env));
        assert_eq!(data, (old, new).into_val(&env));
    }

    // ── fee_deposited ────────────────────────────────────────────────────────

    #[test]
    fn test_emit_fee_deposited() {
        let env = env();
        let market = addr(&env);
        let token = addr(&env);
        emit_fee_deposited(&env, market.clone(), token.clone(), 200_000);

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "fee_deposited"),).into_val(&env));
        assert_eq!(data, (market, token, 200_000_i128).into_val(&env));
    }

    // ── fee_withdrawn ────────────────────────────────────────────────────────

    #[test]
    fn test_emit_fee_withdrawn() {
        let env = env();
        let token = addr(&env);
        let dest = addr(&env);
        emit_fee_withdrawn(&env, token.clone(), 1_000_000, dest.clone());

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "fee_withdrawn"),).into_val(&env));
        assert_eq!(data, (token, 1_000_000_i128, dest).into_val(&env));
    }

    // ── emergency_drain ──────────────────────────────────────────────────────

    #[test]
    fn test_emit_emergency_drain() {
        let env = env();
        let token = addr(&env);
        emit_emergency_drain(&env, token.clone(), 50_000_000);

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "emergency_drain"),).into_val(&env));
        assert_eq!(data, (token, 50_000_000_i128).into_val(&env));
    }

    // ── config_updated ───────────────────────────────────────────────────────

    #[test]
    fn test_emit_config_updated() {
        let env = env();
        emit_config_updated(&env, str(&env, "fee_bps"), 300);

        let (topics, data) = sole_event!(env);
        assert_eq!(topics, (Symbol::new(&env, "config_updated"),).into_val(&env));
        assert_eq!(data, (str(&env, "fee_bps"), 300_i128).into_val(&env));
    }

    // ── conflicting_oracle_report ─────────────────────────────────────────────

    #[test]
    fn test_emit_conflicting_oracle_report() {
        let env = env();
        let oracle = addr(&env);
        emit_conflicting_oracle_report(&env, 10, oracle.clone());

        let (topics, data) = sole_event!(env);
        assert_eq!(
            topics,
            (Symbol::new(&env, "conflicting_oracle_report"), 10_u64).into_val(&env)
        );
        assert_eq!(data, oracle.into_val(&env));
    }
}
