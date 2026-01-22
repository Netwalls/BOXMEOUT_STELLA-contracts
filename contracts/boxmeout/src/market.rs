// Individual Prediction Market Contract - This handles predictions, bet commitment/reveal, market resolution

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, BytesN, Env, Map, Symbol,
    Vec,
};

// Storage keys
const MARKET_ID_KEY: &str = "market_id";
const CREATOR_KEY: &str = "creator";
const FACTORY_KEY: &str = "factory";
const USDC_KEY: &str = "usdc";
const CLOSING_TIME_KEY: &str = "closing_time";
const RESOLUTION_TIME_KEY: &str = "resolution_time";
const MARKET_STATE_KEY: &str = "market_state";
const YES_POOL_KEY: &str = "yes_pool";
const NO_POOL_KEY: &str = "no_pool";
const TOTAL_VOLUME_KEY: &str = "total_volume";
const PENDING_COUNT_KEY: &str = "pending_count";
const COMMIT_PREFIX: &str = "commit";

/// Market states
const STATE_OPEN: u32 = 0;
const STATE_CLOSED: u32 = 1;
const STATE_RESOLVED: u32 = 2;

/// Error codes following Soroban best practices
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MarketError {
    /// Market is not in the required state
    InvalidMarketState = 1,
    /// Action attempted after closing time
    MarketClosed = 2,
    /// Invalid amount (must be positive)
    InvalidAmount = 3,
    /// User has already committed to this market
    DuplicateCommit = 4,
    /// Token transfer failed
    TransferFailed = 5,
    /// Market has not been initialized
    NotInitialized = 6,
}

/// Commitment record for commit-reveal scheme
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Commitment {
    pub user: Address,
    pub commit_hash: BytesN<32>,
    pub amount: i128,
    pub timestamp: u64,
}

/// PREDICTION MARKET - Manages individual market logic
#[contract]
pub struct PredictionMarket;

#[contractimpl]
impl PredictionMarket {
    /// Initialize a single market instance
    pub fn initialize(
        env: Env,
        market_id: BytesN<32>,
        creator: Address,
        factory: Address,
        usdc_token: Address,
        closing_time: u64,
        resolution_time: u64,
    ) {
        // Verify creator signature
        creator.require_auth();

        // Store market_id reference
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, MARKET_ID_KEY), &market_id);

        // Store creator address
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, CREATOR_KEY), &creator);

        // Store factory address
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, FACTORY_KEY), &factory);

        // Store USDC token address
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, USDC_KEY), &usdc_token);

        // Store timing
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, CLOSING_TIME_KEY), &closing_time);

        env.storage()
            .persistent()
            .set(&Symbol::new(&env, RESOLUTION_TIME_KEY), &resolution_time);

        // Initialize market state as open
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, MARKET_STATE_KEY), &STATE_OPEN);

        // Initialize prediction pools
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, YES_POOL_KEY), &0i128);

        env.storage()
            .persistent()
            .set(&Symbol::new(&env, NO_POOL_KEY), &0i128);

        // Initialize total volume
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, TOTAL_VOLUME_KEY), &0i128);

        // Initialize pending count
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, PENDING_COUNT_KEY), &0u32);

        // Emit initialization event
        env.events().publish(
            (Symbol::new(&env, "market_initialized"),),
            (market_id, creator, factory, closing_time, resolution_time),
        );
    }

    pub fn commit_prediction(
        env: Env,
        user: Address,
        commit_hash: BytesN<32>,
        amount: i128,
    ) -> Result<(), MarketError> {
        //  Require user authentication
        user.require_auth();

        //  Validate market is initialized
        let market_state: u32 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, MARKET_STATE_KEY))
            .ok_or(MarketError::NotInitialized)?;

        //  Validate market is in open state
        if market_state != STATE_OPEN {
            return Err(MarketError::InvalidMarketState);
        }

        //  Validate current timestamp < closing_time
        let closing_time: u64 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, CLOSING_TIME_KEY))
            .ok_or(MarketError::NotInitialized)?;

        let current_time = env.ledger().timestamp();
        if current_time >= closing_time {
            return Err(MarketError::MarketClosed);
        }

        //  Validate amount > 0
        if amount <= 0 {
            return Err(MarketError::InvalidAmount);
        }

        //  Check for duplicate commit per user
        let commit_key = Self::get_commit_key(&env, &user);
        if env.storage().persistent().has(&commit_key) {
            return Err(MarketError::DuplicateCommit);
        }

        //  Get USDC token contract and market_id
        let usdc_token: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, USDC_KEY))
            .ok_or(MarketError::NotInitialized)?;

        let market_id: BytesN<32> = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, MARKET_ID_KEY))
            .ok_or(MarketError::NotInitialized)?;

        //  Transfer USDC from user to market escrow (this contract)
        let token_client = token::TokenClient::new(&env, &usdc_token);
        let contract_address = env.current_contract_address();

        // Transfer tokens - will panic if insufficient balance or approval
        token_client.transfer(&user, &contract_address, &amount);

        //  Create and store commitment record
        let commitment = Commitment {
            user: user.clone(),
            commit_hash: commit_hash.clone(),
            amount,
            timestamp: current_time,
        };

        env.storage().persistent().set(&commit_key, &commitment);

        //  Update pending count
        let pending_count: u32 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, PENDING_COUNT_KEY))
            .unwrap_or(0);

        env.storage()
            .persistent()
            .set(&Symbol::new(&env, PENDING_COUNT_KEY), &(pending_count + 1));

        // 11. Emit CommitmentMade event
        env.events().publish(
            (Symbol::new(&env, "CommitmentMade"),),
            (user, market_id, amount),
        );

        Ok(())
    }

    /// Helper: Generate storage key for user commitment
    fn get_commit_key(env: &Env, user: &Address) -> (Symbol, Address) {
        // Use tuple key: (commit_prefix, user_address)
        (Symbol::new(env, COMMIT_PREFIX), user.clone())
    }

    /// Helper: Get user commitment (for testing and reveal phase)
    pub fn get_commitment(env: Env, user: Address) -> Option<Commitment> {
        let commit_key = Self::get_commit_key(&env, &user);
        env.storage().persistent().get(&commit_key)
    }

    /// Helper: Get pending commit count
    pub fn get_pending_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&Symbol::new(&env, PENDING_COUNT_KEY))
            .unwrap_or(0)
    }

    /// Helper: Get market state
    pub fn get_market_state_value(env: Env) -> Option<u32> {
        env.storage()
            .persistent()
            .get(&Symbol::new(&env, MARKET_STATE_KEY))
    }

    /// Phase 2: User reveals their committed prediction
    ///
    /// TODO: Reveal Prediction
    /// - Require user authentication
    /// - Validate market state still OPEN (revelation period)
    /// - Validate user has prior commit record for this market
    /// - Reconstruct commit hash from: outcome + amount + salt provided
    /// - Compare reconstructed hash with stored commit hash
    /// - If hashes don't match: reject with "Invalid revelation"
    /// - Lock in prediction: outcome and amount
    /// - Mark commit as revealed
    /// - Update prediction pool: if outcome==YES: yes_pool+=amount, else: no_pool+=amount
    /// - Calculate odds: yes_odds = yes_pool / (yes_pool + no_pool)
    /// - Store prediction record in user_predictions map
    /// - Remove from pending_commits
    /// - Emit PredictionRevealed(user, market_id, outcome, amount, timestamp)
    /// - Update market total_volume += amount
    pub fn reveal_prediction(
        env: Env,
        user: Address,
        market_id: BytesN<32>,
        outcome: u32,
        amount: i128,
        salt: BytesN<32>,
    ) {
        todo!("See reveal prediction TODO above")
    }

    /// Close market for new predictions (auto-trigger at closing_time)
    ///
    /// TODO: Close Market
    /// - Validate current timestamp >= closing_time
    /// - Validate market state is OPEN
    /// - Change market state to CLOSED
    /// - Lock all remaining balances (no new predictions/trades)
    /// - Freeze prediction pools (finalize odds)
    /// - Emit MarketClosed(market_id, final_yes_pool, final_no_pool, timestamp)
    /// - Record closing timestamp
    pub fn close_market(env: Env, market_id: BytesN<32>) {
        todo!("See close market TODO above")
    }

    /// Resolve market based on oracle consensus result
    ///
    /// TODO: Resolve Market
    /// - Validate current timestamp >= resolution_time
    /// - Validate market state is CLOSED
    /// - Receive oracle_result (0=NO, 1=YES) from oracle module
    /// - Validate oracle_result in [0, 1]
    /// - Set winning_outcome = oracle_result
    /// - Change market state to RESOLVED
    /// - Calculate payouts for winners
    /// - For each winner: payout = (their_amount / total_winners_amount) * total_pool
    /// - Deduct platform fee (10%) from each winner payout
    /// - Store calculated payouts in market state
    /// - Mark market as settled
    /// - Emit MarketResolved(market_id, winning_outcome, total_winners, timestamp)
    /// - Prepare treasury transfers for fee collection
    pub fn resolve_market(env: Env, market_id: BytesN<32>, winning_outcome: u32) {
        todo!("See resolve market TODO above")
    }

    /// Dispute market resolution within 7-day window
    ///
    /// TODO: Dispute Market
    /// - Require user authentication and user participated in market
    /// - Validate market state is RESOLVED
    /// - Validate current timestamp < resolution_time + 7 days
    /// - Store dispute record: { user, reason, timestamp }
    /// - Change market state to DISPUTED
    /// - Freeze all payouts until dispute resolved
    /// - Increment dispute counter
    /// - Emit MarketDisputed(user, reason, market_id, timestamp)
    /// - Notify admin of dispute
    pub fn dispute_market(env: Env, user: Address, market_id: BytesN<32>, dispute_reason: Symbol) {
        todo!("See dispute market TODO above")
    }

    /// Claim winnings after market resolution
    ///
    /// TODO: Claim Winnings
    /// - Require user authentication
    /// - Validate market state is RESOLVED (not DISPUTED)
    /// - Query user's prediction for this market
    /// - Validate prediction exists
    /// - Check user hasn't already claimed
    /// - Validate user's outcome matches winning_outcome
    /// - Calculate user's payout (winning amount - fee)
    /// - Transfer payout from market escrow to user wallet
    /// - Handle USDC transfer failure: log error, don't mark as claimed
    /// - Mark prediction as claimed
    /// - Record claim timestamp
    /// - Emit WinningsClaimed(user, market_id, payout_amount, timestamp)
    /// - Update user stats: wins++, total_winnings += payout
    pub fn claim_winnings(env: Env, user: Address, market_id: BytesN<32>) -> i128 {
        todo!("See claim winnings TODO above")
    }

    /// Refund users if their prediction failed (optional opt-in)
    ///
    /// TODO: Refund Losing Bet
    /// - Require user authentication
    /// - Validate market state is RESOLVED
    /// - Query user's prediction for this market
    /// - Validate user's outcome != winning_outcome (they lost)
    /// - Validate hasn't already been refunded
    /// - Calculate partial refund (e.g., 5% back to incentivize)
    /// - Transfer refund from treasury to user
    /// - Mark as refunded
    /// - Emit LosingBetRefunded(user, market_id, refund_amount, timestamp)
    pub fn refund_losing_bet(env: Env, user: Address, market_id: BytesN<32>) -> i128 {
        todo!("See refund losing bet TODO above")
    }

    /// Get market summary data
    ///
    /// TODO: Get Market State
    /// - Query market metadata from storage
    /// - Return: market_id, creator, category, title, description
    /// - Include timing: creation_time, closing_time, resolution_time, time_remaining
    /// - Include current state: OPEN/CLOSED/RESOLVED/DISPUTED
    /// - Include pools: yes_volume, no_volume, total_volume
    /// - Include odds: yes_odds, no_odds
    /// - Include resolution: winning_outcome (if resolved), timestamp
    /// - Include user-specific data if user provided: their prediction, potential winnings
    pub fn get_market_state(env: Env, market_id: BytesN<32>) -> Symbol {
        todo!("See get market state TODO above")
    }

    /// Get prediction records for a user in this market
    ///
    /// TODO: Get User Prediction
    /// - Query user_predictions map by user + market_id
    /// - Return prediction data: outcome, amount, committed, revealed, claimed
    /// - Include: commit timestamp, reveal timestamp, claim timestamp
    /// - Include potential payout if market is unresolved
    /// - Handle: user has no prediction (return error)
    pub fn get_user_prediction(env: Env, user: Address, market_id: BytesN<32>) -> Symbol {
        todo!("See get user prediction TODO above")
    }

    /// Get all predictions in market (for governance/audits)
    ///
    /// TODO: Get All Predictions
    /// - Require admin or oracle role
    /// - Return list of all user predictions
    /// - Include: user address, outcome, amount for each
    /// - Include participation count and total_volume
    /// - Exclude: user private data (privacy-preserving)
    pub fn get_all_predictions(env: Env, market_id: BytesN<32>) -> Vec<Symbol> {
        todo!("See get all predictions TODO above")
    }

    /// Get market leaderboard (top predictors by winnings)
    ///
    /// TODO: Get Market Leaderboard
    /// - Collect all winners for this market
    /// - Sort by payout amount descending
    /// - Limit top 100
    /// - Return: user address, prediction, payout, accuracy
    /// - For display on frontend
    pub fn get_market_leaderboard(env: Env, market_id: BytesN<32>) -> Vec<Symbol> {
        todo!("See get market leaderboard TODO above")
    }

    /// Get total volume and liquidity for market
    ///
    /// TODO: Get Market Liquidity
    /// - Query yes_pool, no_pool, total_volume
    /// - Calculate current odds for YES and NO
    /// - Return depth: how much can be bought at current price
    /// - Include slippage estimates for trades
    pub fn get_market_liquidity(env: Env, market_id: BytesN<32>) -> i128 {
        todo!("See get market liquidity TODO above")
    }

    /// Emergency function: Market creator can cancel unresolved market
    ///
    /// TODO: Cancel Market (Creator Only)
    /// - Require market creator authentication
    /// - Validate market state is OPEN or CLOSED (not resolved)
    /// - Return all user USDC balances (full refund)
    /// - Loop through all users with predictions
    /// - Transfer their full amounts back from escrow
    /// - Handle any transfer failures (log but continue)
    /// - Set market state to CANCELLED
    /// - Emit MarketCancelled(market_id, reason, creator, timestamp)
    pub fn cancel_market(env: Env, creator: Address, market_id: BytesN<32>) {
        todo!("See cancel market TODO above")
    }
}
