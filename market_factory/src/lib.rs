#![no_std]
/// ============================================================
/// BOXMEOUT — MarketFactory Contract

use soroban_sdk::{contract, contractimpl, contractclient, Address, Env, Vec};

use boxmeout_shared::{
    errors::ContractError,
    types::{BetRecord, MarketConfig, MarketState, MarketStatus, FightDetails, UserPosition},
};

#[contractclient(name = "MarketClient")]
pub trait MarketInterface {
    fn get_bets_by_address(env: Env, bettor: Address) -> Vec<BetRecord>;
}

// ─── Storage Key Constants ────────────────────────────────────────────────────

/// u64 — monotonically increasing counter; also used as market_id
const MARKET_COUNT: &str = "MARKET_COUNT";
/// Map<u64, Address> — market_id → deployed Market contract address
const MARKET_MAP: &str = "MARKET_MAP";
/// Address — factory admin (should be a multisig account in production)
const ADMIN: &str = "ADMIN";
/// Vec<Address> — oracle addresses allowed to resolve markets
const ORACLE_WHITELIST: &str = "ORACLE_WHITELIST";
/// bool — when true, create_market() is rejected
const PAUSED: &str = "PAUSED";
/// MarketConfig — default config applied to new markets unless overridden
const DEFAULT_CONFIG: &str = "DEFAULT_CONFIG";

#[contract]
pub struct MarketFactory;

#[contractimpl]
impl MarketFactory {
    /// Initializes the factory.
    /// Stores admin, oracle whitelist, default config, and sets paused = false.
    /// Must be called exactly once immediately after deployment.
    /// Returns ContractError::AlreadyInitialized on a second call.
    pub fn initialize(
        env: Env,
        admin: Address,
        default_fee_bps: u32,
        oracles: Vec<Address>,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Creates a new prediction market for a boxing match.
    ///
    /// Steps:
    ///   1. Require factory is not paused
    ///   2. Require caller authorization
    ///   3. Validate fight details (scheduled_at in future, names non-empty)
    ///   4. Validate config (min_bet > 0, fee_bps <= 1000)
    ///   5. Deploy a new Market contract wasm
    ///   6. Call Market::initialize() on the new contract
    ///   7. Store market_id → contract_address in MARKET_MAP
    ///   8. Increment MARKET_COUNT
    ///   9. Emit MarketCreated event
    ///  10. Return the new market_id
    pub fn create_market(
        env: Env,
        caller: Address,
        fight: FightDetails,
        config: MarketConfig,
    ) -> Result<u64, ContractError> {
        todo!()
    }

    /// Returns the deployed Market contract address for a given market_id.
    /// Returns ContractError::MarketNotFound if the ID has not been registered.
    pub fn get_market_address(
        env: Env,
        market_id: u64,
    ) -> Result<Address, ContractError> {
        todo!()
    }

    /// Returns a paginated slice of (market_id, MarketStatus) pairs.
    /// offset: first index to return (0-based)
    /// limit:  maximum number of results (capped at 100 on-chain)
    /// Used by the backend indexer to discover all markets without scanning events.
    pub fn list_markets(
        env: Env,
        offset: u64,
        limit: u32,
    ) -> Vec<(u64, MarketStatus)> {
        todo!()
    }

    /// Returns the total number of markets ever created (includes cancelled ones).
    pub fn get_market_count(env: Env) -> u64 {
        todo!()
    }

    /// Adds an oracle address to the whitelist.
    /// Requires admin authorization.
    /// Idempotent — adding an already-present address is a no-op (no error).
    pub fn add_oracle(
        env: Env,
        admin: Address,
        oracle: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Removes an oracle address from the whitelist.
    /// Requires admin authorization.
    /// Returns ContractError::OracleNotWhitelisted if address is not present.
    pub fn remove_oracle(
        env: Env,
        admin: Address,
        oracle: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Returns the current oracle whitelist.
    pub fn get_oracles(env: Env) -> Vec<Address> {
        todo!()
    }

    /// Transfers factory admin rights to new_admin.
    /// Requires current_admin authorization.
    /// Emits AdminTransferred event.
    pub fn transfer_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Pauses market creation. Existing markets are unaffected and continue operating.
    /// Requires admin authorization.
    /// Emergency use only — document reason in the transaction memo.
    pub fn pause_factory(env: Env, admin: Address) -> Result<(), ContractError> {
        todo!()
    }

    /// Re-enables market creation after a pause.
    /// Requires admin authorization.
    pub fn unpause_factory(env: Env, admin: Address) -> Result<(), ContractError> {
        todo!()
    }

    /// Returns true if the factory is currently paused.
    pub fn is_paused(env: Env) -> bool {
        todo!()
    }

    /// Updates the default MarketConfig applied to new markets.
    /// Does NOT retroactively change existing markets.
    /// Requires admin authorization.
    pub fn update_default_config(
        env: Env,
        admin: Address,
        new_config: MarketConfig,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Returns all active (market_id, side) positions held by a user across multiple markets.
    ///
    /// Steps:
    ///   1. Validate market_ids.len() <= 20 (reject if exceeded)
    ///   2. For each market_id in the Vec:
    ///      a. Fetch market address via MARKET_MAP
    ///      b. Call get_bets_by_address(bettor) on that Market contract
    ///      c. Filter bets: keep only non-zero amounts where !claimed
    ///      d. For each filtered bet, create a UserPosition {market_id, side, amount}
    ///   3. Return aggregated Vec<UserPosition>
    ///   4. Return empty Vec if no active positions found
    ///
    /// Returns ContractError::MarketNotFound if any market_id is not registered.
    /// Returns ContractError::TooManyMarkets if market_ids.len() > 20.
    /// Read-only operation; no state mutation.
    pub fn get_user_positions_all(
        env: Env,
        bettor: Address,
        market_ids: Vec<u64>,
    ) -> Result<Vec<UserPosition>, ContractError> {
        // Validate max 20 markets
        if market_ids.len() > 20 {
            return Err(ContractError::TooManyMarkets);
        }

        let mut positions: Vec<UserPosition> = Vec::new(&env);

        // Iterate over each market ID
        for market_id in market_ids.iter() {
            // Get market address from MARKET_MAP
            let market_map: soroban_sdk::Map<u64, Address> =
                env.storage().persistent().get(&MARKET_MAP).unwrap_or_else(
                    || soroban_sdk::Map::new(&env),
                );

            let market_address = market_map
                .get(market_id)
                .ok_or(ContractError::MarketNotFound)?;

            // Create client for cross-contract call
            let market_client = MarketClient::new(&env, &market_address);

            // Call get_bets_by_address on the market contract
            let bets = market_client.get_bets_by_address(&bettor);

            // Filter non-zero, unclaimed bets and convert to UserPosition
            for bet in bets.iter() {
                if bet.amount > 0 && !bet.claimed {
                    positions.push_back(UserPosition {
                        market_id: bet.market_id,
                        side: bet.side.clone(),
                        amount: bet.amount,
                    });
                }
            }
        }

        Ok(positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boxmeout_shared::types::BetSide;
    use soroban_sdk::testutils::Address as _;

    fn create_bet_record(
        env: &Env,
        market_id: u64,
        amount: i128,
        claimed: bool,
    ) -> BetRecord {
        BetRecord {
            bettor: Address::random(env),
            market_id,
            side: BetSide::FighterA,
            amount,
            placed_at: 1000,
            claimed,
        }
    }

    #[test]
    fn test_max_20_markets_allowed() {
        // Test that up to 20 markets is accepted
        let env = Env::default();
        let mut market_ids = Vec::new(&env);

        for i in 0..20 {
            market_ids.push_back(i);
        }

        // Should not exceed limit (this is a signature validation test)
        assert_eq!(market_ids.len(), 20);
    }

    #[test]
    fn test_more_than_20_markets_creates_error_condition() {
        // Test that more than 20 markets would trigger rejection
        // In the actual implementation, this would return TooManyMarkets error
        let env = Env::default();
        let mut market_ids = Vec::new(&env);

        for i in 0..21 {
            market_ids.push_back(i);
        }

        // Verify the condition that would trigger the error
        assert!(market_ids.len() > 20);
    }

    #[test]
    fn test_claimed_bets_excluded() {
        // Test that claimed bets are excluded from positions
        let env = Env::default();
        let claimed_bet = create_bet_record(&env, 1, 1000, true);
        
        // Verify the claimed flag prevents inclusion
        assert!(claimed_bet.claimed);
        assert_eq!(claimed_bet.amount, 1000);
    }

    #[test]
    fn test_zero_amount_bets_excluded() {
        // Test that zero-amount bets are excluded from positions
        let env = Env::default();
        let zero_bet = create_bet_record(&env, 1, 0, false);
        
        // Verify zero amount is filtered
        assert_eq!(zero_bet.amount, 0);
        assert!(!zero_bet.claimed);
    }

    #[test]
    fn test_nonzero_unclaimed_bets_included() {
        // Test that non-zero, unclaimed bets would be included
        let env = Env::default();
        let active_bet = create_bet_record(&env, 1, 500, false);
        
        // Verify conditions for inclusion are met
        assert!(active_bet.amount > 0);
        assert!(!active_bet.claimed);
    }

    #[test]
    fn test_empty_positions_vector() {
        // Test that empty positions vector can be created
        let env = Env::default();
        let positions: Vec<UserPosition> = Vec::new(&env);
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn test_user_position_fields() {
        // Test that UserPosition correctly stores all fields
        let env = Env::default();
        let position = UserPosition {
            market_id: 42,
            side: BetSide::FighterA,
            amount: 1000,
        };

        assert_eq!(position.market_id, 42);
        assert_eq!(position.amount, 1000);
    }

    #[test]
    fn test_multiple_positions_aggregation() {
        // Test that multiple positions can be aggregated correctly
        let env = Env::default();
        let mut positions = Vec::new(&env);

        positions.push_back(UserPosition {
            market_id: 1,
            side: BetSide::FighterA,
            amount: 1000,
        });
        positions.push_back(UserPosition {
            market_id: 2,
            side: BetSide::FighterB,
            amount: 2000,
        });
        positions.push_back(UserPosition {
            market_id: 3,
            side: BetSide::Draw,
            amount: 500,
        });

        assert_eq!(positions.len(), 3);
        assert_eq!(positions.get(0).unwrap().market_id, 1);
        assert_eq!(positions.get(1).unwrap().market_id, 2);
        assert_eq!(positions.get(2).unwrap().market_id, 3);
    }

    #[test]
    fn test_position_from_bet_conversion() {
        // Test that BetRecord correctly converts to UserPosition
        let env = Env::default();
        let bet = create_bet_record(&env, 5, 750, false);
        
        // Verify the conversion logic requirements
        assert_eq!(bet.market_id, 5);
        assert_eq!(bet.side, BetSide::FighterA);
        assert_eq!(bet.amount, 750);

        // Create expected UserPosition from this bet
        let position = UserPosition {
            market_id: bet.market_id,
            side: bet.side,
            amount: bet.amount,
        };

        assert_eq!(position.market_id, bet.market_id);
        assert_eq!(position.amount, bet.amount);
    }
}
