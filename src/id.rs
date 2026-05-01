use rand::Rng;
use sha2::{Digest, Sha256};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use anyhow::Result;

const BASE36_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// Calculate optimal hash length using the birthday problem formula.
///
/// For a given expected number of items N and hash length L with base B:
///   P(collision) ≈ 1 - exp(-N² / (2 * B^L))
///
/// Solving for L given target collision probability p:
///   L ≈ log_B(N² / (2 * -ln(1-p)))
///
/// We use p = 0.05 (5% collision probability) to balance br compatibility
/// with mathematical soundness, and clamp L to [3, 8].
pub fn optimal_hash_length(existing_count: usize) -> usize {
    if existing_count == 0 {
        return 3;
    }

    // Target: 5% collision probability (balance between br compatibility and safety)
    const TARGET_COLLISION_PROB: f64 = 0.05;
    let neg_ln_one_minus_p = -((1.0 - TARGET_COLLISION_PROB).ln());

    // B = 36 (base36), solve for L
    // L = log_36(N² / (2 * -ln(1-p)))
    let n = existing_count as f64;
    let numerator = n * n;
    let denominator = 2.0 * neg_ln_one_minus_p;
    let base = 36.0_f64;

    let length = (numerator / denominator).log(base).ceil() as usize;

    // Clamp to [3, 8] range
    length.clamp(3, 8)
}

pub fn base36_encode(data: &[u8]) -> String {
    let mut result = String::new();
    let num = BigUint::from_bytes_be(data);
    let base = BigUint::from(36u32);

    if num.is_zero() {
        return "0".to_string();
    }

    let mut n = num;
    while n > BigUint::zero() {
        let remainder = &n % &base;
        let digit = remainder.to_u32_digits().first().copied().unwrap_or(0) as usize;
        result.insert(0, BASE36_CHARS[digit] as char);
        n /= &base;
    }

    result
}

pub fn generate_id(prefix: &str, existing_count: usize) -> String {
    let len = optimal_hash_length(existing_count);
    let random_bytes: [u8; 16] = rand::thread_rng().gen();
    let hash = Sha256::digest(&random_bytes);

    // Use all 32 bytes of the SHA-256 hash for maximum entropy,
    // then truncate the base36-encoded result to the desired length.
    let hash_encoded = base36_encode(&hash);
    let truncated = hash_encoded.chars().take(len).collect::<String>();

    format!("{}-{}", prefix, truncated)
}

pub fn is_valid_bead_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('-').collect();
    if parts.len() < 2 {
        return false;
    }
    let hash_part = parts[1..].join("");
    // Check that hash part is not empty and contains only alphanumeric chars
    !hash_part.is_empty() && hash_part.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_hash_length() {
        // Values based on birthday problem formula with 5% collision target
        assert_eq!(optimal_hash_length(0), 3);
        assert_eq!(optimal_hash_length(100), 4);
        assert_eq!(optimal_hash_length(500), 4);
        assert_eq!(optimal_hash_length(1000), 5);
        assert_eq!(optimal_hash_length(5000), 5);
        assert_eq!(optimal_hash_length(10000), 6);
        assert_eq!(optimal_hash_length(50000), 6);
        assert_eq!(optimal_hash_length(100000), 7);
    }

    #[test]
    fn test_generate_id() {
        let id = generate_id("bf", 100);
        assert!(id.starts_with("bf-"));
        assert!(is_valid_bead_id(&id));
    }

    #[test]
    fn test_is_valid_bead_id() {
        assert!(is_valid_bead_id("bf-abc123"));
        assert!(is_valid_bead_id("bd-a1b2c3"));
        assert!(!is_valid_bead_id("invalid"));
        assert!(!is_valid_bead_id("bf-"));
    }

    #[test]
    fn test_base36_encode() {
        let result = base36_encode(&[0xFF, 0xFF]);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_br_format_compatibility() {
        // IDs should be lowercase alphanumeric after the prefix
        for _ in 0..100 {
            let id = generate_id("bf", 100);
            let hash_part = id.split('-').nth(1).unwrap();
            assert!(hash_part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                    "Hash should be lowercase alphanumeric: {}", id);
        }
    }

    #[test]
    fn test_adaptive_hash_length() {
        // Verify hash length changes with count
        let id_100 = generate_id("bf", 100);
        let hash_100 = id_100.split('-').nth(1).unwrap();
        assert_eq!(hash_100.len(), optimal_hash_length(100));

        let id_10000 = generate_id("bf", 10000);
        let hash_10000 = id_10000.split('-').nth(1).unwrap();
        assert_eq!(hash_10000.len(), optimal_hash_length(10000));

        // Higher count should produce longer or equal hash
        assert!(hash_10000.len() >= hash_100.len());
    }

    #[test]
    fn test_no_collisions_10k() {
        // Acceptance criteria: no collisions in 10k-ID corpus
        let mut ids = std::collections::HashSet::new();

        for i in 0..10000 {
            let id = generate_id("bf", i);
            assert!(ids.insert(id.clone()), "Collision detected at {}: {}", i, id);
        }

        assert_eq!(ids.len(), 10000, "Should generate 10000 unique IDs");
    }
}
