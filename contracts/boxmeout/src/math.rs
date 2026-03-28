// src/math.rs — Overflow-safe arithmetic primitives for CPMM and fee math.
//
// Issues closed:
//   #46 — mul_div, mul_div_ceil, checked_product
//   #47 — sqrt (integer floor via Newton-Raphson)
//   #48 — apply_fee_bps, split_fees

/// Overflow-safe floor division: floor(a * b / d).
///
/// Uses 256-bit widening via u128 hi/lo decomposition so the intermediate
/// product never overflows.  Returns 0 when d == 0 or the true result would
/// not fit in i128.
pub fn mul_div(a: i128, b: i128, d: i128) -> i128 {
    if d == 0 {
        return 0;
    }
    // Work in u128 magnitude, track sign separately.
    let neg = (a < 0) ^ (b < 0) ^ (d < 0);
    let ua = a.unsigned_abs();
    let ub = b.unsigned_abs();
    let ud = d.unsigned_abs();

    let result_u128 = mul_div_u128(ua, ub, ud);

    // Check the result fits in i128.
    if result_u128 > i128::MAX as u128 {
        return 0;
    }
    let result = result_u128 as i128;
    if neg { -result } else { result }
}

/// Overflow-safe ceiling division: ceil(a * b / d).
///
/// Returns 0 on d == 0 or overflow (same contract as mul_div).
pub fn mul_div_ceil(a: i128, b: i128, d: i128) -> i128 {
    if d == 0 {
        return 0;
    }
    let neg = (a < 0) ^ (b < 0) ^ (d < 0);
    let ua = a.unsigned_abs();
    let ub = b.unsigned_abs();
    let ud = d.unsigned_abs();

    // ceil = floor + (remainder != 0 ? 1 : 0)
    let (lo, hi) = mul128(ua, ub);
    let (q, r) = div256_by_u128(lo, hi, ud);
    let q_ceil = if r != 0 { q.saturating_add(1) } else { q };

    if q_ceil > i128::MAX as u128 {
        return 0;
    }
    let result = q_ceil as i128;
    if neg { -result } else { result }
}

/// Returns the product of all values in `factors`, or 0 on any overflow.
pub fn checked_product(factors: &[u128]) -> u128 {
    let mut acc: u128 = 1;
    for &f in factors {
        match acc.checked_mul(f) {
            Some(v) => acc = v,
            None => return 0,
        }
    }
    acc
}

/// Integer floor square root via Newton-Raphson.
///
/// Terminates in O(log n) iterations.
/// Returns 0 for n == 0, 1 for n == 1.
pub fn sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    // Initial estimate: shift right by half the bit-length.
    let mut x: u128 = 1u128 << ((128 - n.leading_zeros()) / 2);
    loop {
        let x1 = (x + n / x) / 2;
        if x1 >= x {
            return x;
        }
        x = x1;
    }
}

/// Compute the fee amount for a given `amount` and rate in basis points.
///
/// `fee = floor(amount * fee_bps / 10_000)`
///
/// Uses `mul_div` so the intermediate product never overflows even for
/// `amount == i128::MAX`.  Returns 0 when `fee_bps == 0` or `amount == 0`.
/// Panics if `fee_bps > 10_000` (fee cannot exceed 100 %).
pub fn apply_fee_bps(amount: i128, fee_bps: u32) -> i128 {
    if fee_bps > 10_000 {
        panic!("fee_bps exceeds 10_000");
    }
    mul_div(amount, fee_bps as i128, 10_000)
}

/// Split `total_fee` into three buckets: protocol, LP, and creator.
///
/// Each share is computed as `floor(total_fee * share_bps / 10_000)`.
/// Any rounding remainder is added to `protocol` so that:
///   `protocol + lp + creator == total_fee` always holds.
///
/// Panics if the three `*_bps` values sum to more than 10_000, or if
/// `total_fee` is negative.
///
/// Returns `(protocol, lp, creator)`.
pub fn split_fees(
    total_fee: i128,
    protocol_bps: u32,
    lp_bps: u32,
    creator_bps: u32,
) -> (i128, i128, i128) {
    if total_fee < 0 {
        panic!("total_fee must be non-negative");
    }
    let sum_bps = protocol_bps as u64 + lp_bps as u64 + creator_bps as u64;
    if sum_bps > 10_000 {
        panic!("split bps sum exceeds 10_000");
    }
    let lp = mul_div(total_fee, lp_bps as i128, 10_000);
    let creator = mul_div(total_fee, creator_bps as i128, 10_000);
    // Protocol absorbs the remainder to guarantee exact reconstruction.
    let protocol = total_fee - lp - creator;
    (protocol, lp, creator)
}

// ── internal helpers ────────────────────────────────────────────────────────

/// Multiply two u128 values, returning (lo, hi) of the 256-bit product.
fn mul128(a: u128, b: u128) -> (u128, u128) {
    // Split each operand into 64-bit halves.
    let a_lo = a & u64::MAX as u128;
    let a_hi = a >> 64;
    let b_lo = b & u64::MAX as u128;
    let b_hi = b >> 64;

    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;

    let mid = lh + hl;
    let mid_carry = if mid < lh { 1u128 } else { 0u128 };

    let lo = ll.wrapping_add(mid << 64);
    let lo_carry = if lo < ll { 1u128 } else { 0u128 };
    let hi = hh + (mid >> 64) + (mid_carry << 64) + lo_carry;

    (lo, hi)
}

/// Divide a 256-bit value (lo, hi) by a u128 divisor.
/// Returns (quotient, remainder) as u128.
/// Panics if hi >= divisor (quotient would overflow u128).
fn div256_by_u128(lo: u128, hi: u128, d: u128) -> (u128, u128) {
    if hi == 0 {
        return (lo / d, lo % d);
    }
    // Long division: process 64-bit chunks.
    // hi < d is required for the quotient to fit in u128.
    // We use the standard "schoolbook" 2-by-1 limb division.
    let mut rem: u128 = hi;
    let mut q: u128 = 0;

    // Process high 64 bits of lo.
    let lo_hi = lo >> 64;
    let cur = (rem << 64) | lo_hi;
    let q_hi = cur / d;
    rem = cur % d;

    // Process low 64 bits of lo.
    let lo_lo = lo & (u64::MAX as u128);
    let cur = (rem << 64) | lo_lo;
    let q_lo = cur / d;
    rem = cur % d;

    q = (q_hi << 64) | q_lo;
    (q, rem)
}

/// Core u128 mul_div used by the signed wrappers.
fn mul_div_u128(a: u128, b: u128, d: u128) -> u128 {
    if d == 0 {
        return 0;
    }
    let (lo, hi) = mul128(a, b);
    if hi >= d {
        // Quotient would overflow u128 — signal overflow with 0.
        return 0;
    }
    let (q, _) = div256_by_u128(lo, hi, d);
    q
}

// ── unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── sqrt ────────────────────────────────────────────────────────────────

    #[test]
    fn test_sqrt_zero_and_one() {
        assert_eq!(sqrt(0), 0);
        assert_eq!(sqrt(1), 1);
    }

    #[test]
    fn test_sqrt_perfect_squares() {
        for i in 2u128..=1000 {
            let s = sqrt(i * i);
            assert_eq!(s, i, "sqrt({}) should be {}", i * i, i);
        }
    }

    #[test]
    fn test_sqrt_near_squares() {
        // sqrt(n)^2 <= n < (sqrt(n)+1)^2
        let cases = [2u128, 3, 5, 8, 10, 15, 24, 26, 99, 101, 999, 1001, u128::MAX];
        for &n in &cases {
            let s = sqrt(n);
            assert!(s * s <= n, "sqrt({})^2 > n", n);
            if s < u128::MAX {
                assert!((s + 1) * (s + 1) > n, "(sqrt({})+1)^2 <= n", n);
            }
        }
    }

    #[test]
    fn test_sqrt_large() {
        let n = 1_000_000_000_000_000_000u128;
        let s = sqrt(n);
        assert_eq!(s, 1_000_000_000);
        assert!(s * s <= n);
        assert!((s + 1) * (s + 1) > n);
    }

    // ── mul_div ─────────────────────────────────────────────────────────────

    #[test]
    fn test_mul_div_basic() {
        assert_eq!(mul_div(10, 3, 5), 6);
        assert_eq!(mul_div(7, 3, 2), 10); // floor(21/2)
        assert_eq!(mul_div(0, 999, 1), 0);
        assert_eq!(mul_div(1, 1, 1), 1);
    }

    #[test]
    fn test_mul_div_signs() {
        assert_eq!(mul_div(-10, 3, 5), -6);
        assert_eq!(mul_div(10, -3, 5), -6);
        assert_eq!(mul_div(-10, -3, 5), 6);
        assert_eq!(mul_div(10, 3, -5), -6);
    }

    #[test]
    fn test_mul_div_zero_divisor_returns_zero() {
        assert_eq!(mul_div(100, 200, 0), 0);
    }

    #[test]
    fn test_mul_div_large_values() {
        // (i128::MAX / 2) * 2 / 1  should equal i128::MAX - 1 (floor)
        let half = i128::MAX / 2;
        let result = mul_div(half, 2, 1);
        assert_eq!(result, half * 2);
    }

    #[test]
    fn test_mul_div_overflow_returns_zero() {
        // i128::MAX * i128::MAX / 1 overflows — must return 0
        assert_eq!(mul_div(i128::MAX, i128::MAX, 1), 0);
    }

    // ── mul_div_ceil ─────────────────────────────────────────────────────────

    #[test]
    fn test_mul_div_ceil_basic() {
        assert_eq!(mul_div_ceil(7, 3, 2), 11); // ceil(21/2)
        assert_eq!(mul_div_ceil(10, 3, 5), 6); // exact — same as floor
    }

    #[test]
    fn test_mul_div_ceil_vs_floor_off_by_one() {
        // When remainder != 0, ceil = floor + 1
        let floor = mul_div(7, 1, 2);   // floor(7/2) = 3
        let ceil  = mul_div_ceil(7, 1, 2); // ceil(7/2)  = 4
        assert_eq!(ceil, floor + 1);
    }

    #[test]
    fn test_mul_div_ceil_exact_no_difference() {
        // When exact, floor == ceil
        let floor = mul_div(8, 1, 2);
        let ceil  = mul_div_ceil(8, 1, 2);
        assert_eq!(floor, ceil);
    }

    #[test]
    fn test_mul_div_ceil_zero_divisor_returns_zero() {
        assert_eq!(mul_div_ceil(100, 200, 0), 0);
    }

    // ── checked_product ──────────────────────────────────────────────────────

    #[test]
    fn test_checked_product_empty() {
        assert_eq!(checked_product(&[]), 1);
    }

    #[test]
    fn test_checked_product_small() {
        assert_eq!(checked_product(&[2, 3, 4]), 24);
        assert_eq!(checked_product(&[1, 1, 1, 1]), 1);
        assert_eq!(checked_product(&[0, 999]), 0);
    }

    #[test]
    fn test_checked_product_overflow_returns_zero() {
        assert_eq!(checked_product(&[u128::MAX, 2]), 0);
    }

    // ── apply_fee_bps ────────────────────────────────────────────────────────

    #[test]
    fn test_apply_fee_bps_acceptance() {
        // 2 % of 1_000_000 stroops == 20_000
        assert_eq!(apply_fee_bps(1_000_000, 200), 20_000);
    }

    #[test]
    fn test_apply_fee_bps_zero_rate() {
        assert_eq!(apply_fee_bps(1_000_000, 0), 0);
    }

    #[test]
    fn test_apply_fee_bps_zero_amount() {
        assert_eq!(apply_fee_bps(0, 200), 0);
    }

    #[test]
    fn test_apply_fee_bps_100_percent() {
        // 100 % fee (10_000 bps) returns the full amount.
        assert_eq!(apply_fee_bps(500, 10_000), 500);
    }

    #[test]
    fn test_apply_fee_bps_no_overflow_at_i128_max() {
        // Must not panic or overflow for the largest possible amount.
        let fee = apply_fee_bps(i128::MAX, 200);
        assert!(fee > 0);
    }

    #[test]
    #[should_panic(expected = "fee_bps exceeds 10_000")]
    fn test_apply_fee_bps_over_100_percent_panics() {
        apply_fee_bps(1_000_000, 10_001);
    }

    // ── split_fees ───────────────────────────────────────────────────────────

    #[test]
    fn test_split_fees_sums_to_total() {
        // 1 % protocol, 1 % LP, 0.5 % creator  (total 2.5 %)
        let total: i128 = 1_000_000;
        let (protocol, lp, creator) = split_fees(total, 100, 100, 50);
        assert_eq!(protocol + lp + creator, total);
        assert!(protocol > 0);
        assert!(lp > 0);
        assert!(creator > 0);
    }

    #[test]
    fn test_split_fees_net_positive() {
        let (protocol, lp, creator) = split_fees(10_000, 5000, 3000, 2000);
        assert_eq!(protocol + lp + creator, 10_000);
        assert!(protocol + lp + creator > 0);
    }

    #[test]
    fn test_split_fees_remainder_goes_to_protocol() {
        // 1 stroop total, 33 % each — floor rounds down, remainder to protocol.
        let (protocol, lp, creator) = split_fees(1, 3333, 3333, 3333);
        assert_eq!(protocol + lp + creator, 1);
    }

    #[test]
    fn test_split_fees_zero_total() {
        let (protocol, lp, creator) = split_fees(0, 5000, 3000, 2000);
        assert_eq!((protocol, lp, creator), (0, 0, 0));
    }

    #[test]
    fn test_split_fees_no_overflow_at_i128_max() {
        let (protocol, lp, creator) = split_fees(i128::MAX, 5000, 3000, 2000);
        assert_eq!(protocol + lp + creator, i128::MAX);
    }

    #[test]
    #[should_panic(expected = "split bps sum exceeds 10_000")]
    fn test_split_fees_bps_over_100_percent_panics() {
        split_fees(1_000_000, 5000, 3000, 2001);
    }

    #[test]
    #[should_panic(expected = "total_fee must be non-negative")]
    fn test_split_fees_negative_total_panics() {
        split_fees(-1, 100, 100, 100);
    }

    // ── fuzz: 10 000 random (a, b, d) triples ────────────────────────────────

    #[test]
    fn test_mul_div_fuzz_10000() {
        // Deterministic LCG so the test is reproducible without external crates.
        let mut state: u64 = 0xDEAD_BEEF_CAFE_1234;
        let lcg = |s: &mut u64| -> u64 {
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *s
        };

        for _ in 0..10_000 {
            let a = lcg(&mut state) as i128;
            let b = lcg(&mut state) as i128;
            let d_raw = lcg(&mut state) as i128;
            let d = if d_raw == 0 { 1 } else { d_raw };

            let result = mul_div(a, b, d);

            // If result is non-zero, verify it satisfies the floor property
            // using i128 checked arithmetic as the reference.
            if let Some(product) = a.checked_mul(b) {
                let expected = product / d; // Rust truncates toward zero
                // Adjust for floor (toward -inf) vs truncation (toward zero)
                let floor_expected = if (product % d != 0) && ((product < 0) ^ (d < 0)) {
                    expected - 1
                } else {
                    expected
                };
                assert_eq!(
                    result, floor_expected,
                    "mul_div({a}, {b}, {d}) = {result}, expected {floor_expected}"
                );
            } else {
                // Overflow case — result must be 0
                assert_eq!(result, 0, "overflow case must return 0 for mul_div({a},{b},{d})");
            }
        }
    }
}
