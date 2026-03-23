// Bloom Filter — Split Block design with 8 hash functions per block
//
// Matches MongoDB's SplitBlockBloomFilter (sbe/util/bloom_filter.h):
// - Each block is 32 bytes (256 bits) = one cache line
// - 8 hash functions derived from one input hash via salt multiplication
// - All 8 probes hit the same cache line -> excellent locality
// - Power-of-2 block count for fast modulo via bitmask
//
// Used in MongoDB's HybridHashJoinStage:
//   Build phase -> insert all build-side keys into bloom filter
//   Probe phase -> skip probe rows that definitely don't match (false negative = impossible)
//
// Properties:
//   - No false negatives: if `insert(x)` was called, `maybe_contains(x)` = true always
//   - False positives: `maybe_contains(y)` can return true even if y was never inserted
//   - FP rate controlled by sizing: more bits per element -> lower FP rate
//
// Complexity: insert O(1), `maybe_contains` O(1), space O(n / ln(2)^2 * 1/fpRate)

use std::hash::{BuildHasher, Hash};

#[derive(Debug)]
pub struct BloomFilter {
    blocks: Vec<Block>,
}

/// 32-byte block aligned to cache line. 8 words x 4 bytes = 32 bytes.
#[repr(align(32))]
#[derive(Debug)]
struct Block {
    words: [u32; 8],
}

/// Salt constants for deriving 8 hash functions from one hash value.
/// Each salt produces a different bit position within the block.
const SALT: [u32; 8] = [
    0x47b6_137b,
    0x4497_4d91,
    0x8824_ad5b,
    0xa2b7_289d,
    0x7054_95c7,
    0x2df1_424b,
    0x9efc_4947,
    0x5c6b_fb31,
];

impl BloomFilter {
    /// Create with target false positive rate for expected number of elements.
    /// Formula: bits = -8n / ln(1 - `fpRate`^(1/8)) where 8 = number of hash functions.
    /// O(n / `fpRate`) to allocate blocks.
    ///
    /// # Panics
    ///
    /// Panics if `expected_elements` is 0, or if `false_positive_rate` is not in (0, 1).
    #[must_use]
    pub fn new(expected_elements: usize, false_positive_rate: f64) -> Self {
        assert!(expected_elements > 0, "expected_elements must be > 0");
        assert!(
            false_positive_rate > 0.0 && false_positive_rate < 1.0,
            "false_positive_rate must be in (0, 1)"
        );
        // Intentional precision loss: element count fits in f64 mantissa for practical sizes.
        #[allow(clippy::cast_precision_loss)]
        let num_bits_f =
            -8.0 * expected_elements as f64 / (1.0 - false_positive_rate.powf(1.0 / 8.0)).ln();
        // Intentional truncation and sign loss: result is always non-negative.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let num_bits = num_bits_f as usize;
        let num_blocks = num_bits.div_ceil(256).next_power_of_two().max(1);
        let blocks = (0..num_blocks).map(|_| Block { words: [0; 8] }).collect();
        Self { blocks }
    }

    fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Insert a pre-hashed value. Call with `hash(key)`, not key directly.
    /// O(1) -- 8 bit-set operations within a single cache line.
    pub fn insert(&mut self, hash: u64) {
        let block_idx = (hash >> 32) as usize & (self.num_blocks() - 1);
        // Intentional truncation: we only want the lower 32 bits of the hash.
        #[allow(clippy::cast_possible_truncation)]
        let h = hash as u32;
        for (i, &salt) in SALT.iter().enumerate() {
            // Top 5 bits of (h * SALT[i]) -> bit position 0..31
            let bit = h.wrapping_mul(salt) >> 27;
            self.blocks[block_idx].words[i] |= 1 << bit;
        }
    }

    /// Check if a value was possibly inserted. No false negatives.
    /// O(1) -- 8 bit-check operations within a single cache line.
    #[must_use]
    pub fn maybe_contains(&self, hash: u64) -> bool {
        let block_idx = (hash >> 32) as usize & (self.num_blocks() - 1);
        // Intentional truncation: we only want the lower 32 bits of the hash.
        #[allow(clippy::cast_possible_truncation)]
        let h = hash as u32;
        let block = &self.blocks[block_idx];
        SALT.iter().enumerate().all(|(i, &salt)| {
            let bit = h.wrapping_mul(salt) >> 27;
            block.words[i] & (1 << bit) != 0
        })
    }
}

/// Helper: hash any hashable value to `u64` for use with `BloomFilter`.
/// Callers must use the same `BuildHasher` instance for both insert and lookup.
/// O(size of val) -- one hash computation.
#[must_use]
pub fn compute_hash<T: Hash>(val: &T, hash_builder: &impl BuildHasher) -> u64 {
    hash_builder.hash_one(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::RandomState;

    // ZERO
    #[test]
    fn empty_filter_rejects() {
        let s = RandomState::new();
        let bf = BloomFilter::new(100, 0.01);
        assert!(!bf.maybe_contains(compute_hash(&42, &s)));
    }

    // ONE
    #[test]
    fn single_element() {
        let s = RandomState::new();
        let mut bf = BloomFilter::new(10, 0.01);
        bf.insert(compute_hash(&42, &s));
        assert!(bf.maybe_contains(compute_hash(&42, &s)));
        assert!(!bf.maybe_contains(compute_hash(&99, &s)));
    }

    // MANY -- no false negatives
    #[test]
    fn inserted_elements_found() {
        let s = RandomState::new();
        let mut bf = BloomFilter::new(100, 0.01);
        for i in 0..100 {
            bf.insert(compute_hash(&i, &s));
        }
        for i in 0..100 {
            assert!(
                bf.maybe_contains(compute_hash(&i, &s)),
                "False negative for {}",
                i
            );
        }
    }

    // EDGE: false positive rate within bounds
    #[test]
    fn false_positive_rate_bounded() {
        let s = RandomState::new();
        let mut bf = BloomFilter::new(1000, 0.01);
        for i in 0..1000 {
            bf.insert(compute_hash(&i, &s));
        }
        let fps: usize = (1000..11000)
            .filter(|i| bf.maybe_contains(compute_hash(i, &s)))
            .count();
        // With 1% target FP rate, expect <200 out of 10000 (2% margin for randomness)
        assert!(fps < 200, "FP rate too high: {}/10000", fps);
    }
}
