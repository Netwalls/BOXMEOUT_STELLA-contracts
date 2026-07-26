//! =============================================================================
//! BOXMEOUT — Property-Based Fuzz Tests (proptest)
//! =============================================================================
//!
//! Feeds randomized bet sequences into the Market contract to confirm no
//! overflow, panic, or invariant violations under realistic and adversarial
//! inputs.
//!
//! Runs in CI with a bounded number of test cases (configurable via
//! `PROPTEST_CASES` env var, default 256).

use market::types::{
    BetSide, Fighter, Market, MarketStatus, Outcome, ProtocolConfig,
};
use market::{DataKey, MarketContract, MarketContractClient};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger},
    Address, Bytes, Env, String, Symbol, Vec,
};

// ─── Mock Factory ─────────────────────────────────────────────────────────────

#[contract]
struct MockFactory;

#[contractimpl]
impl MockFactory {
    pub fn __constructor(env: Env, admin: Address) {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "admin"), &admin);
    }

    pub fn get_config(env: Env) -> ProtocolConfig {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "admin"))
            .unwrap();
        ProtocolConfig {
            admin: admin.clone(),
            fee_collector: admin,
            default_fee_bp: 200,
            min_bet_amount: 1,
            max_bet_amount: 100_000_000_000,
            dispute_window_sec: 86_400,
            paused: false,
        }
    }
}

// ─── Proptest strategies ──────────────────────────────────────────────────────

mod proptest_helpers {
    use proptest::prelude::*;

    /// Generates a bet amount within a safe range for testing (1 .. 10_000_000_000).
    pub fn bet_amount() -> impl Strategy<Value = i128> {
        1i128..10_000_000_000i128
    }

    /// Generates a bet side (true = FighterA, false = FighterB).
    pub fn bet_side() -> impl Strategy<Value = bool> {
        any::<bool>()
    }

    /// Generates a sequence of (amount, is_fighter_a) pairs.
    pub fn bet_sequence(max_len: usize) -> impl Strategy<Value = Vec<(i128, bool)>> {
        proptest::collection::vec((bet_amount(), bet_side()), 1..max_len)
    }
}

// ─── Helper ───────────────────────────────────────────────────────────────────

fn make_fighter(env: &Env, name: &str) -> Fighter {
    Fighter {
        name: String::from_str(env, name),
        record: String::from_str(env, "10-0"),
        nationality: String::from_str(env, "US"),
        weight_class: String::from_str(env, "Heavyweight"),
    }
}

// ─── Fuzz: place bets only ────────────────────────────────────────────────────

proptest::proptest! {
    /// Random sequences of bets must never cause a panic or overflow during
    /// `place_bet`.  We don't resolve — we only exercise the betting path.
    #[test]
    fn fuzz_place_bet_no_panic(
        bets in proptest_helpers::bet_sequence(50),
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let factory_id = env.register(MockFactory, (admin.clone(),));
        let oracle = Address::generate(&env);
        let fee_collector = Address::generate(&env);

        let now = env.ledger().timestamp();
        let betting_ends_at = now + 10_000_000;

        let market_cid = env.register(MarketContract, ());
        let client = MarketContractClient::new(&env, &market_cid);

        client.initialize(
            &Bytes::from_array(&env, &[1u8; 32]),
            &make_fighter(&env, "Alpha"),
            &make_fighter(&env, "Beta"),
            &(betting_ends_at + 1_000_000),
            &betting_ends_at,
            &oracle,
            &factory_id,
            &200u32,
            &fee_collector,
        );

        for (amount, is_a) in &bets {
            let side = if *is_a { BetSide::FighterA } else { BetSide::FighterB };
            let bettor = Address::generate(&env);
            let _bet_id = client.place_bet(&bettor, &side, &amount);

            // Basic invariant: total_pool >= pool_a + pool_b
            let m = client.get_market_info();
            assert!(
                m.total_pool >= m.pool_a.checked_add(m.pool_b).unwrap_or(0),
                "total_pool must be >= pool_a + pool_b"
            );
        }
    }
}

proptest::proptest! {
    /// Full lifecycle with random bets: place, lock, resolve, and verify all
    /// payouts sum to net pool (total_pool - fee).
    #[test]
    fn fuzz_full_lifecycle_payout_invariant(
        bets in proptest_helpers::bet_sequence(30),
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let factory_id = env.register(MockFactory, (admin.clone(),));
        let oracle = Address::generate(&env);
        let fee_collector = Address::generate(&env);

        let now = env.ledger().timestamp();
        let betting_ends_at = now + 10_000_000;

        let market_cid = env.register(MarketContract, ());
        let client = MarketContractClient::new(&env, &market_cid);

        client.initialize(
            &Bytes::from_array(&env, &[2u8; 32]),
            &make_fighter(&env, "Alpha"),
            &make_fighter(&env, "Beta"),
            &(betting_ends_at + 1_000_000),
            &betting_ends_at,
            &oracle,
            &factory_id,
            &200u32,
            &fee_collector,
        );

        // Track (bettor, bet_id, amount) for claims later
        let mut bets_on_a: Vec<(Address, Bytes, i128)> = Vec::new(&env);
        let mut bets_on_b: Vec<(Address, Bytes, i128)> = Vec::new(&env);

        for (amount, is_a) in &bets {
            let side = if *is_a { BetSide::FighterA } else { BetSide::FighterB };
            let bettor = Address::generate(&env);
            let bet_id = client.place_bet(&bettor, &side, &amount);

            match side {
                BetSide::FighterA => bets_on_a.push_back((bettor, bet_id, *amount)),
                BetSide::FighterB => bets_on_b.push_back((bettor, bet_id, *amount)),
            }
        }

        let market = client.get_market_info();
        let total_pool = market.total_pool;

        // Fast-forward past betting deadline, then lock and resolve
        env.ledger().with_mut(|l| l.timestamp = betting_ends_at + 1);

        // Manually lock storage (lock_market is a todo!() currently)
        env.as_contract(&market_cid, || {
            let mut m: Market = env
                .storage()
                .persistent()
                .get(&DataKey::MarketInfo)
                .unwrap();
            m.status = MarketStatus::Locked;
            env.storage().persistent().set(&DataKey::MarketInfo, &m);
        });

        // Resolve with FighterA winning
        client.resolve_market(&oracle, &Outcome::FighterA);

        // Claim all winning bets
        let mut total_claimed = 0i128;
        for (bettor, bet_id, _amount) in bets_on_a.iter() {
            let payout = client.claim_winnings(&bettor, &bet_id);
            total_claimed = total_claimed.checked_add(payout).expect("claim sum overflow");
        }

        // Invariant: total_claimed + fee == total_pool (within rounding)
        let expected_fee = total_pool
            .checked_mul(200)
            .expect("fee mul overflow")
            .checked_div(10_000)
            .expect("fee div");
        assert!(
            total_claimed <= total_pool,
            "claims must not exceed total pool"
        );
        assert!(
            total_claimed + expected_fee >= total_pool - (bets_on_a.len() as i128),
            "claimed + fee should be close to total pool"
        );
    }
}

proptest::proptest! {
    /// Verify get_pool_odds never panics and always returns values in [0, 10000].
    #[test]
    fn fuzz_pool_odds_no_panic(
        bets in proptest_helpers::bet_sequence(20),
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let factory_id = env.register(MockFactory, (admin.clone(),));
        let oracle = Address::generate(&env);
        let fee_collector = Address::generate(&env);

        let now = env.ledger().timestamp();
        let betting_ends_at = now + 10_000_000;

        let market_cid = env.register(MarketContract, ());
        let client = MarketContractClient::new(&env, &market_cid);

        client.initialize(
            &Bytes::from_array(&env, &[3u8; 32]),
            &make_fighter(&env, "Alpha"),
            &make_fighter(&env, "Beta"),
            &(betting_ends_at + 1_000_000),
            &betting_ends_at,
            &oracle,
            &factory_id,
            &200u32,
            &fee_collector,
        );

        for (amount, is_a) in &bets {
            let side = if *is_a { BetSide::FighterA } else { BetSide::FighterB };
            let bettor = Address::generate(&env);
            let _ = client.place_bet(&bettor, &side, &amount);
        }

        let (pa, pb, odds_a, odds_b) = client.get_pool_odds();

        assert!(odds_a <= 10_000, "odds_a out of range: {}", odds_a);
        assert!(odds_b <= 10_000, "odds_b out of range: {}", odds_b);
        if pa + pb > 0 {
            assert_eq!(odds_a + odds_b, 10_000, "odds must sum to 10000");
        }
    }
}
