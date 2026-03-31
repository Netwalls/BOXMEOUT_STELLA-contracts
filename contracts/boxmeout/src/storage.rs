// src/storage.rs — Soroban persistent-storage TTL constants and helpers.
//
// Soroban charges rent on persistent entries. Every write must be paired with
// an extend_ttl call so entries survive long enough to be useful.
//
// Ledger cadence on Stellar mainnet: ~5 seconds per ledger.
//   1 day  ≈  17_280 ledgers
//   1 year ≈  6_307_200 ledgers

use soroban_sdk::{Env, IntoVal, TryFromVal, Val};

// ---------------------------------------------------------------------------
// TTL constants (in ledgers)
// ---------------------------------------------------------------------------

/// Minimum TTL threshold: bump when remaining TTL falls below this.
/// Set to ~30 days so we don't bump on every single read.
pub const TTL_THRESHOLD: u32 = 518_400; // 30 days

/// Target TTL after a bump: keep entries alive for ~1 year.
pub const TTL_TARGET: u32 = 6_307_200; // 1 year

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Write `value` to persistent storage under `key` and immediately extend
/// its TTL to [`TTL_TARGET`] ledgers (bumping only when below [`TTL_THRESHOLD`]).
///
/// Every `env.storage().persistent().set(...)` call in the contract should
/// use this function instead of calling `.set(...)` directly.
pub fn set_and_bump<K, V>(env: &Env, key: &K, value: &V)
where
    K: IntoVal<Env, Val> + TryFromVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().persistent().set(key, value);
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD, TTL_TARGET);
}
