// contracts/amm.rs - Automated Market Maker for Outcome Shares
// Enables trading YES/NO outcome shares with dynamic odds pricing (Polymarket model)

use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Symbol, Vec};

use crate::{amm, helpers::*};

// Storage keys
const ADMIN_KEY: &str = "admin";
const FACTORY_KEY: &str = "factory";
const USDC_KEY: &str = "usdc";
const MAX_LIQUIDITY_CAP_KEY: &str = "max_liquidity_cap";
const SLIPPAGE_PROTECTION_KEY: &str = "slippage_protection";
const TRADING_FEE_KEY: &str = "trading_fee";
const PRICING_MODEL_KEY: &str = "pricing_model";

// Pool storage keys
const POOL_YES_RESERVE_KEY: &str = "pool_yes_reserve";
const POOL_NO_RESERVE_KEY: &str = "pool_no_reserve";
const POOL_EXISTS_KEY: &str = "pool_exists";

// Pool data structure
#[derive(Clone)]
pub struct Pool {
    pub yes_reserve: u128,
    pub no_reserve: u128,
    pub total_liquidity: u128,
    pub created_at: u64,
}

// Helper function to create pool storage key
fn pool_key(market_id: &BytesN<32>, suffix: &str) -> Symbol {
    let env = &market_id.env();
    let mut key_str = String::new();

    // Convert market_id bytes to hex string
    for byte in market_id.as_slice() {
        key_str.push_str(&format!("{:02x}", byte));
    }
    key_str.push_str("_");
    key_str.push_str(suffix);

    Symbol::new(env, &key_str)
}

/// AUTOMATED MARKET MAKER - Manages liquidity pools and share trading
#[contract]
pub struct AMM;

#[contractimpl]
impl AMM {
    /// Initialize AMM with liquidity pools
    pub fn initialize(
        env: Env,
        admin: Address,
        factory: Address,
        usdc_token: Address,
        max_liquidity_cap: u128,
    ) {
        // Verify admin signature
        admin.require_auth();

        // Store admin address
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, ADMIN_KEY), &admin);

        // Store factory address
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, FACTORY_KEY), &factory);

        // Store USDC token contract address
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, USDC_KEY), &usdc_token);

        // Set max_liquidity_cap per market
        env.storage().persistent().set(
            &Symbol::new(&env, MAX_LIQUIDITY_CAP_KEY),
            &max_liquidity_cap,
        );

        // Set slippage_protection default (2% = 200 basis points)
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, SLIPPAGE_PROTECTION_KEY), &200u32);

        // Set trading fee (0.2% = 20 basis points)
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, TRADING_FEE_KEY), &20u32);

        // Set pricing_model (CPMM - Constant Product Market Maker)
        env.storage().persistent().set(
            &Symbol::new(&env, PRICING_MODEL_KEY),
            &Symbol::new(&env, "CPMM"),
        );

        // Emit initialization event
        env.events().publish(
            (Symbol::new(&env, "amm_initialized"),),
            (admin, factory, max_liquidity_cap),
        );
    }

    /// Create new liquidity pool for market
    pub fn create_pool(env: Env, market_id: BytesN<32>, initial_liquidity: u128) {
        // Check if pool already exists
        let pool_exists_key = pool_key(&market_id, POOL_EXISTS_KEY);
        if env.storage().persistent().has(&pool_exists_key) {
            panic!("pool already exists");
        }

        // Validate initial liquidity
        if initial_liquidity == 0 {
            panic!("initial liquidity must be greater than 0");
        }

        // Initialize 50/50 split
        let yes_reserve = initial_liquidity / 2;
        let no_reserve = initial_liquidity / 2;

        // Calculate constant product k = x * y
        let k = yes_reserve * no_reserve;

        // Create storage keys for this pool using tuples
        let yes_reserve_key = (Symbol::new(&env, POOL_YES_RESERVE_PREFIX), &market_id);
        let no_reserve_key = (Symbol::new(&env, POOL_NO_RESERVE_PREFIX), &market_id);
        let k_key = (Symbol::new(&env, POOL_K_PREFIX), &market_id);
        let lp_supply_key = (Symbol::new(&env, POOL_LP_SUPPLY_PREFIX), &market_id);
        let lp_balance_key = (
            Symbol::new(&env, POOL_LP_TOKENS_PREFIX),
            &market_id,
            &creator,
        );

        // Store reserves
        env.storage()
            .persistent()
            .set(&yes_reserve_key, &yes_reserve);
        env.storage().persistent().set(&no_reserve_key, &no_reserve);
        env.storage().persistent().set(&k_key, &k);

        // Mark pool as existing
        env.storage().persistent().set(&pool_exists_key, &true);

        // Mint LP tokens to creator (equal to initial_liquidity for first LP)
        let lp_tokens = initial_liquidity;
        env.storage().persistent().set(&lp_supply_key, &lp_tokens);
        env.storage().persistent().set(&lp_balance_key, &lp_tokens);

        // Transfer USDC from creator to contract
        let usdc_token: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, USDC_KEY))
            .expect("usdc token not set");

        let token_client = token::Client::new(&env, &usdc_token);
        token_client.transfer(
            &creator,
            &env.current_contract_address(),
            &(initial_liquidity as i128),
        );

        // Calculate initial odds (50/50)
        let yes_odds = 5000u32; // 50.00%
        let no_odds = 5000u32; // 50.00%

        // Emit PoolCreated event
        env.events().publish(
            (Symbol::new(&env, "pool_created"),),
            (market_id, initial_liquidity, yes_reserve, no_reserve),
        );
    }

    /// Buy outcome shares (YES or NO)
    /// Uses Constant Product Market Maker (CPMM) formula: x * y = k
    /// Returns number of shares purchased
    pub fn buy_shares(
        env: Env,
        buyer: Address,
        market_id: BytesN<32>,
        outcome: u32,
        amount: u128,
        min_shares: u128,
    ) -> u128 {
        // Require buyer authentication
        buyer.require_auth();

        // Validate inputs
        if outcome > 1 {
            panic!("outcome must be 0 (NO) or 1 (YES)");
        }
        if amount == 0 {
            panic!("amount must be greater than 0");
        }

        // Check if pool exists
        let pool_exists_key = pool_key(&market_id, POOL_EXISTS_KEY);
        if !env.storage().persistent().has(&pool_exists_key) {
            panic!("pool does not exist");
        }

        // Get current reserves
        let yes_key = pool_key(&market_id, POOL_YES_RESERVE_KEY);
        let no_key = pool_key(&market_id, POOL_NO_RESERVE_KEY);

        let yes_reserve: u128 = env.storage().persistent().get(&yes_key).unwrap_or(0);
        let no_reserve: u128 = env.storage().persistent().get(&no_key).unwrap_or(0);

        if yes_reserve == 0 || no_reserve == 0 {
            panic!("insufficient liquidity");
        }

        // Calculate trading fee (20 basis points = 0.2%)
        let trading_fee_bps: u128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, TRADING_FEE_KEY))
            .unwrap_or(20);

        let fee_amount = (amount * trading_fee_bps) / 10000;
        let amount_after_fee = amount - fee_amount;

        // CPMM calculation: shares_out = (amount_in * reserve_out) / (reserve_in + amount_in)
        let (reserve_in, reserve_out, new_reserve_in, new_reserve_out) = if outcome == 1 {
            // Buying YES shares: pay with USDC, get YES shares
            // Input reserve is NO (what we're paying with conceptually)
            // Output reserve is YES (what we're getting)
            let shares_out = (amount_after_fee * yes_reserve) / (no_reserve + amount_after_fee);
            (
                no_reserve,
                yes_reserve,
                no_reserve + amount_after_fee,
                yes_reserve - shares_out,
            )
        } else {
            // Buying NO shares: pay with USDC, get NO shares
            let shares_out = (amount_after_fee * no_reserve) / (yes_reserve + amount_after_fee);
            (
                yes_reserve,
                no_reserve,
                yes_reserve + amount_after_fee,
                no_reserve - shares_out,
            )
        };

        let shares_out = if outcome == 1 {
            (amount_after_fee * reserve_out) / (reserve_in + amount_after_fee)
        } else {
            (amount_after_fee * reserve_out) / (reserve_in + amount_after_fee)
        };

        // Slippage protection
        if shares_out < min_shares {
            panic!(
                "Slippage exceeded: would receive {} shares, minimum is {}",
                shares_out, min_shares
            );
        }

        // Verify CPMM invariant (k should increase due to fees)
        let old_k = yes_reserve * no_reserve;
        let new_k = new_reserve_in * new_reserve_out;
        if new_k < old_k {
            panic!("invariant violation");
        }

        // Update reserves
        if outcome == 1 {
            // Bought YES: increase NO reserve, decrease YES reserve
            env.storage()
                .persistent()
                .set(&no_key, &(no_reserve + amount_after_fee));
            env.storage()
                .persistent()
                .set(&yes_key, &(yes_reserve - shares_out));
        } else {
            // Buying NO: NO reserve decreases by shares_out, YES reserve increases by input
            (yes_reserve + amount_after_fee, no_reserve - shares_out)
        };

        set_pool_reserves(&env, &market_id, new_yes_reserve, new_no_reserve);

        let current_shares = get_user_shares(&env, &buyer, &market_id, outcome);

        set_user_shares(
            &env,
            &buyer,
            &market_id,
            outcome,
            current_shares + shares_out,
        );

        let trade_index = increment_trade_count(&env, &market_id);
        let trade_key = (Symbol::new(&env, "trade"), market_id.clone(), trade_index);
        env.storage().persistent().set(
            &trade_key,
            &(
                buyer.clone(),
                outcome,
                shares_out,
                amount,
                fee,
                env.ledger().timestamp(),
            ),
        );

        env.events().publish(
            (Symbol::new(&env, "BuyShares"),),
            (buyer, market_id, outcome, shares_out, amount, fee),
        );

        shares_out
    }

    /// Sell outcome shares back to AMM
    /// Returns USDC payout amount
    pub fn sell_shares(
        env: Env,
        seller: Address,
        market_id: BytesN<32>,
        outcome: u32,
        shares: u128,
        min_payout: u128,
    ) -> u128 {
        seller.require_auth();

        if outcome > 1 {
            panic!("Invalid outcome: must be 0 (NO) or 1 (YES)");
        }
        if shares == 0 {
            panic!("Shares execution amount must be positive");
        }

        if !pool_exists(&env, &market_id) {
            panic!("Liquidity pool does not exist");
        }

        // Check user balance
        let user_shares = get_user_shares(&env, &seller, &market_id, outcome);
        if user_shares < shares {
            panic!("Insufficient shares balance");
        }

        let (yes_reserve, no_reserve) = get_pool_reserves(&env, &market_id);

        // Calculate raw payout using reverse CPMM
        let payout = calculate_payout(yes_reserve, no_reserve, outcome, shares);

        // Apply fee (0.2%)
        let trading_fee_bps: u32 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, TRADING_FEE_KEY))
            .unwrap_or(20);

        let fee = payout * (trading_fee_bps as u128) / 10_000;
        let payout_after_fee = payout - fee;

        // Check slippage
        if payout_after_fee < min_payout {
            panic!(
                "Slippage exceeded: would receive {} USDC, minimum is {}",
                payout_after_fee, min_payout
            );
        }

        // Update reserves
        // If selling YES: YES reserve increases by shares, NO reserve decreases by payout
        let (new_yes_reserve, new_no_reserve) = if outcome == 1 {
            (yes_reserve + shares, no_reserve - payout)
        } else {
            // If selling NO: NO reserve increases by shares, YES reserve decreases by payout
            (yes_reserve - payout, no_reserve + shares)
        };

        set_pool_reserves(&env, &market_id, new_yes_reserve, new_no_reserve);

        // Burn user shares
        set_user_shares(&env, &seller, &market_id, outcome, user_shares - shares);

        // Transfer USDC to seller
        let usdc_address: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, USDC_KEY))
            .expect("USDC token not configured");
        let usdc_client = soroban_sdk::token::Client::new(&env, &usdc_address);

        usdc_client.transfer(
            &env.current_contract_address(),
            &seller,
            &(payout_after_fee as i128),
        );

        // Record trade
        let trade_index = increment_trade_count(&env, &market_id);
        let trade_key = (Symbol::new(&env, "trade"), market_id.clone(), trade_index);
        env.storage().persistent().set(
            &trade_key,
            &(
                seller.clone(),
                outcome,
                shares,           // shares sold
                payout_after_fee, // amount received
                fee,
                env.ledger().timestamp(),
            ),
        );

        // Emit SellShares event
        env.events().publish(
            (Symbol::new(&env, "SellShares"),),
            (seller, market_id, outcome, shares, payout_after_fee, fee),
        );

        payout_after_fee
    }

    /// Calculate current odds for an outcome
    ///
    /// TODO: Get Odds
    /// - Query pool reserves: yes_quantity, no_quantity
    /// - Calculate odds using: outcome_qty / total_qty
    /// - YES_odds = yes_quantity / (yes_quantity + no_quantity)
    /// - NO_odds = no_quantity / (yes_quantity + no_quantity)
    /// - Return as percentage (0.55 = 55%)
    /// - Include implied probability
    pub fn get_odds(env: Env, market_id: BytesN<32>) -> (u128, u128) {
        todo!("See get odds TODO above")
    }

    /// Get current pool state (reserves, liquidity depth)
    ///
    /// TODO: Get Pool State
    /// - Query pool for market_id
    /// - Return: yes_reserve, no_reserve, total_liquidity
    /// - Include: current_odds for both outcomes
    /// - Include: volume_24h, fee_generated_24h
    /// - Include: slippage at different buy amounts
    pub fn get_pool_state(env: Env, market_id: BytesN<32>) -> Symbol {
        todo!("See get pool state TODO above")
    }

    /// Add liquidity to existing pool (become LP)
    ///
    /// Validates pool exists, calculates proportional YES/NO amounts,
    /// updates reserves and k, mints LP tokens proportional to contribution.
    pub fn add_liquidity(
        env: Env,
        lp_provider: Address,
        market_id: BytesN<32>,
        liquidity_amount: u128,
    ) -> u128 {
        // Require LP provider authentication
        lp_provider.require_auth();

        // Validate liquidity_amount > 0
        if liquidity_amount == 0 {
            panic!("liquidity amount must be positive");
        }

        // Check if pool exists
        let pool_exists_key = pool_key(&market_id, POOL_EXISTS_KEY);
        if !env.storage().persistent().has(&pool_exists_key) {
            panic!("pool does not exist");
        }

        // Create storage keys for this pool
        let yes_reserve_key = (Symbol::new(&env, POOL_YES_RESERVE_PREFIX), &market_id);
        let no_reserve_key = (Symbol::new(&env, POOL_NO_RESERVE_PREFIX), &market_id);
        let k_key = (Symbol::new(&env, POOL_K_PREFIX), &market_id);
        let lp_supply_key = (Symbol::new(&env, POOL_LP_SUPPLY_PREFIX), &market_id);
        let lp_balance_key = (
            Symbol::new(&env, POOL_LP_TOKENS_PREFIX),
            &market_id,
            &lp_provider,
        );

        // Get current reserves
        let yes_key = pool_key(&market_id, POOL_YES_RESERVE_KEY);
        let no_key = pool_key(&market_id, POOL_NO_RESERVE_KEY);

        let yes_reserve: u128 = env.storage().persistent().get(&yes_key).unwrap_or(0);
        let no_reserve: u128 = env.storage().persistent().get(&no_key).unwrap_or(0);

        if yes_reserve == 0 || no_reserve == 0 {
            panic!("insufficient liquidity");
        }

        // CPMM calculation for selling: payout = (shares * reserve_out) / (reserve_in + shares)
        let payout = if outcome == 1 {
            // Selling YES shares: get USDC back
            // Input reserve is YES (what we're selling)
            // Output reserve is NO (what we're getting paid from)
            (shares * no_reserve) / (yes_reserve + shares)
        } else {
            // Selling NO shares: get USDC back
            (shares * yes_reserve) / (no_reserve + shares)
        };

        // Calculate trading fee (20 basis points = 0.2%)
        let trading_fee_bps: u128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, TRADING_FEE_KEY))
            .unwrap_or(20);

        let fee_amount = (payout * trading_fee_bps) / 10000;
        let payout_after_fee = payout - fee_amount;

        // Slippage protection
        if payout_after_fee < min_payout {
            panic!("slippage exceeded");
        }

        // Update reserves
        let new_yes_reserve = yes_reserve + yes_addition;
        let new_no_reserve = no_reserve + no_addition;

        // Update k
        let new_k = new_yes_reserve * new_no_reserve;

        // Check max liquidity cap
        let max_liquidity_cap: u128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, MAX_LIQUIDITY_CAP_KEY))
            .expect("max liquidity cap not set");

        let new_total_liquidity = new_yes_reserve + new_no_reserve;
        if new_total_liquidity > max_liquidity_cap {
            panic!("exceeds max liquidity cap");
        }

        // Store updated reserves and k
        env.storage()
            .persistent()
            .set(&yes_reserve_key, &new_yes_reserve);
        env.storage()
            .persistent()
            .set(&no_reserve_key, &new_no_reserve);
        env.storage().persistent().set(&k_key, &new_k);

        // Update LP token supply
        let new_lp_supply = current_lp_supply + lp_tokens_to_mint;
        env.storage()
            .persistent()
            .set(&lp_supply_key, &new_lp_supply);

        // Update LP provider's balance
        let current_lp_balance: u128 = env.storage().persistent().get(&lp_balance_key).unwrap_or(0);
        let new_lp_balance = current_lp_balance + lp_tokens_to_mint;
        env.storage()
            .persistent()
            .set(&lp_balance_key, &new_lp_balance);

        // Transfer USDC from LP provider to contract
        let usdc_token: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, USDC_KEY))
            .expect("usdc token not set");

        let token_client = token::Client::new(&env, &usdc_token);
        token_client.transfer(
            &lp_provider,
            &env.current_contract_address(),
            &(liquidity_amount as i128),
        );

        // Emit LiquidityAdded event
        env.events().publish(
            (Symbol::new(&env, "sell_shares"),),
            (
                seller,
                market_id,
                outcome,
                shares,
                payout_after_fee,
                fee_amount,
            ),
        );

        payout_after_fee
    }

    /// Calculate current odds for an outcome
    /// Returns (yes_odds, no_odds) in basis points (5000 = 50%)
    /// Handles zero-liquidity safely by returning (5000, 5000)
    /// Read-only function with no state changes
    pub fn get_odds(env: Env, market_id: BytesN<32>) -> (u32, u32) {
        // Check if pool exists
        let pool_exists_key = pool_key(&market_id, POOL_EXISTS_KEY);
        if !env.storage().persistent().has(&pool_exists_key) {
            // No pool exists - return 50/50 odds
            return (5000, 5000);
        }
        /// Remove liquidity from pool (redeem LP tokens)
        ///
        /// Validates LP token ownership, calculates proportional YES/NO withdrawal,
        /// burns LP tokens, updates reserves and k, transfers tokens to user.
        pub fn remove_liquidity(
            env: Env,
            lp_provider: Address,
            market_id: BytesN<32>,
            lp_tokens: u128,
        ) -> (u128, u128) {
            // Require LP provider authentication
            lp_provider.require_auth();

            // Validate lp_tokens > 0
            if lp_tokens == 0 {
                panic!("lp tokens must be positive");
            }

            // Check if pool exists for this market
            let pool_exists_key = (Symbol::new(&env, POOL_EXISTS_PREFIX), &market_id);
            if !env.storage().persistent().has(&pool_exists_key) {
                panic!("pool does not exist");
            }

            // Create storage keys for this pool
            let yes_reserve_key = (Symbol::new(&env, POOL_YES_RESERVE_PREFIX), &market_id);
            let no_reserve_key = (Symbol::new(&env, POOL_NO_RESERVE_PREFIX), &market_id);
            let k_key = (Symbol::new(&env, POOL_K_PREFIX), &market_id);
            let lp_supply_key = (Symbol::new(&env, POOL_LP_SUPPLY_PREFIX), &market_id);
            let lp_balance_key = (
                Symbol::new(&env, POOL_LP_TOKENS_PREFIX),
                &market_id,
                &lp_provider,
            );

            // Get LP provider's current balance
            let lp_balance: u128 = env.storage().persistent().get(&lp_balance_key).unwrap_or(0);

            // Validate user has enough LP tokens
            if lp_balance < lp_tokens {
                panic!("insufficient lp tokens");
            }

            // Get current reserves
            let yes_reserve: u128 = env
                .storage()
                .persistent()
                .get(&yes_reserve_key)
                .expect("yes reserve not found");
            let no_reserve: u128 = env
                .storage()
                .persistent()
                .get(&no_reserve_key)
                .expect("no reserve not found");

            // Get current LP token supply
            let current_lp_supply: u128 = env
                .storage()
                .persistent()
                .get(&lp_supply_key)
                .expect("lp supply not found");

            // Calculate proportional YES and NO amounts to withdraw
            // yes_amount = (lp_tokens / current_lp_supply) * yes_reserve
            let yes_amount = (lp_tokens * yes_reserve) / current_lp_supply;
            let no_amount = (lp_tokens * no_reserve) / current_lp_supply;

            if yes_amount == 0 || no_amount == 0 {
                panic!("withdrawal amount too small");
            }

            // Update reserves
            let new_yes_reserve = yes_reserve - yes_amount;
            let new_no_reserve = no_reserve - no_amount;

            // Validate minimum liquidity remains (prevent draining pool completely)
            if new_yes_reserve == 0 || new_no_reserve == 0 {
                panic!("cannot drain pool completely");
            }

            // Update k
            let new_k = new_yes_reserve * new_no_reserve;

            // Store updated reserves and k
            env.storage()
                .persistent()
                .set(&yes_reserve_key, &new_yes_reserve);
            env.storage()
                .persistent()
                .set(&no_reserve_key, &new_no_reserve);
            env.storage().persistent().set(&k_key, &new_k);

            // Burn LP tokens from provider
            let new_lp_balance = lp_balance - lp_tokens;
            if new_lp_balance == 0 {
                env.storage().persistent().remove(&lp_balance_key);
            } else {
                env.storage()
                    .persistent()
                    .set(&lp_balance_key, &new_lp_balance);
            }

            // Update LP token supply
            let new_lp_supply = current_lp_supply - lp_tokens;
            env.storage()
                .persistent()
                .set(&lp_supply_key, &new_lp_supply);

            // Transfer USDC back to user (YES and NO reserves are in USDC)
            // The user receives their proportional share of the pool's liquidity
            let usdc_token: Address = env
                .storage()
                .persistent()
                .get(&Symbol::new(&env, USDC_KEY))
                .expect("usdc token not set");

            let token_client = token::Client::new(&env, &usdc_token);
            let total_withdrawal = yes_amount + no_amount;
            token_client.transfer(
                &env.current_contract_address(),
                &lp_provider,
                &(total_withdrawal as i128),
            );

            // Emit LiquidityRemoved event
            env.events().publish(
                (Symbol::new(&env, "LiquidityRemoved"),),
                (market_id, lp_provider, lp_tokens, yes_amount, no_amount),
            );

            (yes_amount, no_amount)
        }

        // Get pool reserves
        let yes_key = pool_key(&market_id, POOL_YES_RESERVE_KEY);
        let no_key = pool_key(&market_id, POOL_NO_RESERVE_KEY);

        let yes_reserve: u128 = env.storage().persistent().get(&yes_key).unwrap_or(0);
        let no_reserve: u128 = env.storage().persistent().get(&no_key).unwrap_or(0);

        // Handle zero liquidity case
        if yes_reserve == 0 && no_reserve == 0 {
            return (5000, 5000);
        }

        // Handle single-sided liquidity (edge case)
        if yes_reserve == 0 {
            return (0, 10000); // 0% YES, 100% NO
        }
        if no_reserve == 0 {
            return (10000, 0); // 100% YES, 0% NO
        }

        let total_liquidity = yes_reserve + no_reserve;

        // Calculate odds as percentage of total liquidity
        // YES odds = no_reserve / total_liquidity (inverse relationship)
        // NO odds = yes_reserve / total_liquidity (inverse relationship)
        // This follows AMM pricing where higher reserve = lower price

        let yes_odds = ((no_reserve * 10000) / total_liquidity) as u32;
        let no_odds = ((yes_reserve * 10000) / total_liquidity) as u32;

        // Ensure odds sum to 10000 (handle rounding)
        let total_odds = yes_odds + no_odds;
        if total_odds != 10000 {
            let adjustment = 10000 - total_odds;
            if yes_odds >= no_odds {
                return (yes_odds + adjustment, no_odds);
            } else {
                return (yes_odds, no_odds + adjustment);
            }
        }

        (yes_odds, no_odds)
    }

    /// Get current pool state (reserves, liquidity depth)
    /// Returns pool information for frontend display
    pub fn get_pool_state(env: Env, market_id: BytesN<32>) -> (u128, u128, u128, u32, u32) {
        // Check if pool exists
        let pool_exists_key = pool_key(&market_id, POOL_EXISTS_KEY);
        if !env.storage().persistent().has(&pool_exists_key) {
            return (0, 0, 0, 5000, 5000); // No pool: zero reserves, 50/50 odds
        }

        // Get pool reserves
        let yes_key = pool_key(&market_id, POOL_YES_RESERVE_KEY);
        let no_key = pool_key(&market_id, POOL_NO_RESERVE_KEY);

        let yes_reserve: u128 = env.storage().persistent().get(&yes_key).unwrap_or(0);
        let no_reserve: u128 = env.storage().persistent().get(&no_key).unwrap_or(0);
        let total_liquidity = yes_reserve + no_reserve;

        // Get current odds
        let (yes_odds, no_odds) = Self::get_odds(env.clone(), market_id);

        // Return: (yes_reserve, no_reserve, total_liquidity, yes_odds, no_odds)
        (yes_reserve, no_reserve, total_liquidity, yes_odds, no_odds)
    }

    // TODO: Implement remaining AMM functions
    // - add_liquidity() / remove_liquidity()
    // - get_lp_position() / claim_lp_fees()
    // - calculate_spot_price()
    // - get_trade_history()
    // - rebalance_pool()
    // - drain_pool()
    // - get_amm_analytics()
}
