use soroban_sdk::{contracttype, Address, Bytes, String};

// ─── ENUMS ────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MarketStatus {
    Open,       // Bets are being accepted
    Locked,     // Fight started — no more bets
    Resolved,   // Winner declared — claims open
    Cancelled,  // Fight cancelled — full refunds
    Disputed,   // Result under admin review — claims frozen
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    FighterA,   // Fighter A wins
    FighterB,   // Fighter B wins
    Draw,       // Match ends in a draw — status set to Cancelled for full refunds
    NoContest,  // No contest — DQ or injury ruling
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum BetSide {
    FighterA,
    FighterB,
}

// ─── STRUCTS ──────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug)]
pub struct Fighter {
    pub name:         String,
    pub record:       String,
    pub nationality:  String,
    pub weight_class: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Market {
    pub market_id:              Bytes,
    pub fighter_a:              Fighter,
    pub fighter_b:              Fighter,
    pub scheduled_at:           u64,
    pub betting_ends_at:        u64,
    pub created_at:             u64,
    pub created_by:             Address,
    pub status:                 MarketStatus,
    pub pool_a:                 i128,
    pub pool_b:                 i128,
    pub total_pool:             i128,
    pub protocol_fee_bp:        u32,
    pub oracle_address:         Address,
    pub fee_collector_address:  Address,
    pub outcome:                Option<Outcome>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Bet {
    pub bet_id:    Bytes,
    pub market_id: Bytes,
    pub bettor:    Address,
    pub side:      BetSide,
    pub amount:    i128,
    pub placed_at: u64,
    pub claimed:   bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ClaimReceipt {
    pub bet_id:     Bytes,
    pub bettor:     Address,
    pub payout:     i128,
    pub claimed_at: u64,
}

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

// ─── HELPERS ──────────────────────────────────────────────────────────────────

/// Compute protocol fee from a total amount and fee in basis points.
/// Returns (amount * fee_bp) / 10_000.  Uses checked arithmetic to prevent overflow.
pub fn calculate_fee(amount: i128, fee_bp: u32) -> i128 {
    amount
        .checked_mul(fee_bp as i128)
        .expect("fee calculation overflow")
        .checked_div(10_000)
        .expect("fee calculation division error")
}

// ─── TESTS ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::calculate_fee;

    #[test]
    fn fee_zero_bp_returns_zero() {
        assert_eq!(calculate_fee(1_000_000, 0), 0);
    }

    #[test]
    fn fee_100_bp_is_one_percent() {
        assert_eq!(calculate_fee(1_000_000, 100), 10_000);
    }

    #[test]
    fn fee_200_bp_is_two_percent() {
        assert_eq!(calculate_fee(1_000_000, 200), 20_000);
    }

    #[test]
    fn fee_500_bp_is_five_percent() {
        assert_eq!(calculate_fee(1_000_000, 500), 50_000);
    }

    #[test]
    fn fee_10000_bp_returns_full_amount() {
        let amount = 1_000_000_i128;
        assert_eq!(calculate_fee(amount, 10_000), amount);
    }

    #[test]
    fn fee_rounds_down() {
        // 1 stroop * 100 bp = 0 (truncated)
        assert_eq!(calculate_fee(1, 100), 0);
    }
}
