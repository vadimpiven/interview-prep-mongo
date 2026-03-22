// Bloom Filter — Split Block design with 8 hash functions per block
//
// Matches MongoDB's SplitBlockBloomFilter (sbe/util/bloom_filter.h):
// - Each block is 32 bytes (256 bits) = one cache line
// - 8 hash functions derived from one input hash via salt multiplication
// - All 8 probes hit the same cache line → excellent locality
// - Power-of-2 block count for fast modulo via bitmask
//
// Used in MongoDB's HybridHashJoinStage:
//   Build phase → insert all build-side keys into bloom filter
//   Probe phase → skip probe rows that definitely don't match (false negative = impossible)
//
// Properties:
//   - No false negatives: if insert(x) was called, maybe_contains(x) = true always
//   - False positives: maybe_contains(y) can return true even if y was never inserted
//   - FP rate controlled by sizing: more bits per element → lower FP rate
//
// Complexity: insert O(1), maybe_contains O(1), space O(n / ln(2)^2 * 1/fpRate)

use std::hash::{Hash, Hasher};

pub struct BloomFilter {
    blocks: Vec<Block>,
    num_blocks: usize,
}

/// 32-byte block aligned to cache line. 8 words × 4 bytes = 32 bytes.
#[repr(align(32))]
struct Block {
    words: [u32; 8],
}

/// Salt constants for deriving 8 hash functions from one hash value.
/// Each salt produces a different bit position within the block.
const SALT: [u32; 8] = [
    0x47b6137b, 0x44974d91, 0x8824ad5b, 0xa2b7289d, 0x705495c7, 0x2df1424b, 0x9efc4947, 0x5c6bfb31,
];

impl BloomFilter {
    /// Create with target false positive rate for expected number of elements.
    /// Formula: bits = -8n / ln(1 - fpRate^(1/8)) where 8 = number of hash functions.
    pub fn new(expected_elements: usize, fp_rate: f64) -> Self {
        let num_bits =
            (-8.0 * expected_elements as f64 / (1.0 - fp_rate.powf(1.0 / 8.0)).ln()) as usize;
        let num_blocks = ((num_bits + 255) / 256).next_power_of_two().max(1);
        let blocks = (0..num_blocks).map(|_| Block { words: [0; 8] }).collect();
        Self { blocks, num_blocks }
    }

    /// Insert a pre-hashed value. Call with hash(key), not key directly.
    pub fn insert(&mut self, hash: u64) {
        let block_idx = (hash >> 32) as usize & (self.num_blocks - 1);
        let h = hash as u32;
        for i in 0..8 {
            // Top 5 bits of (h * SALT[i]) → bit position 0..31
            let bit = h.wrapping_mul(SALT[i]) >> 27;
            self.blocks[block_idx].words[i] |= 1 << bit;
        }
    }

    /// Check if a value was possibly inserted. No false negatives.
    pub fn maybe_contains(&self, hash: u64) -> bool {
        let block_idx = (hash >> 32) as usize & (self.num_blocks - 1);
        let h = hash as u32;
        let block = &self.blocks[block_idx];
        SALT.iter().enumerate().all(|(i, &salt)| {
            let bit = h.wrapping_mul(salt) >> 27;
            block.words[i] & (1 << bit) != 0
        })
    }
}

/// Helper: hash any hashable value to u64 for use with BloomFilter.
pub fn hash_value<T: Hash>(val: &T) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    val.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_elements_found() {
        let mut bf = BloomFilter::new(100, 0.01);
        for i in 0..100 {
            bf.insert(hash_value(&i));
        }
        // No false negatives allowed
        for i in 0..100 {
            assert!(
                bf.maybe_contains(hash_value(&i)),
                "False negative for {}",
                i
            );
        }
    }

    #[test]
    fn empty_filter_rejects() {
        let bf = BloomFilter::new(100, 0.01);
        assert!(!bf.maybe_contains(hash_value(&42)));
    }

    #[test]
    fn false_positive_rate_bounded() {
        let mut bf = BloomFilter::new(1000, 0.01);
        for i in 0..1000 {
            bf.insert(hash_value(&i));
        }
        let fps: usize = (1000..11000)
            .filter(|i| bf.maybe_contains(hash_value(i)))
            .count();
        // With 1% target FP rate, expect <200 out of 10000 (2% margin for randomness)
        assert!(fps < 200, "FP rate too high: {}/10000", fps);
    }

    #[test]
    fn single_element() {
        let mut bf = BloomFilter::new(10, 0.01);
        bf.insert(hash_value(&42));
        assert!(bf.maybe_contains(hash_value(&42)));
    }

    #[test]
    fn string_keys() {
        let mut bf = BloomFilter::new(100, 0.01);
        bf.insert(hash_value(&"hello"));
        bf.insert(hash_value(&"world"));
        assert!(bf.maybe_contains(hash_value(&"hello")));
        assert!(bf.maybe_contains(hash_value(&"world")));
    }

    #[test]
    fn insert_same_element_twice_is_idempotent() {
        let mut bf = BloomFilter::new(10, 0.01);
        bf.insert(hash_value(&42));
        bf.insert(hash_value(&42));
        assert!(bf.maybe_contains(hash_value(&42)));
    }

    #[test]
    fn minimal_filter_one_element() {
        let mut bf = BloomFilter::new(1, 0.5);
        bf.insert(hash_value(&1));
        assert!(bf.maybe_contains(hash_value(&1)));
    }

    #[test]
    fn many_elements_no_false_negatives() {
        let mut bf = BloomFilter::new(10000, 0.001);
        for i in 0..10000 {
            bf.insert(hash_value(&i));
        }
        for i in 0..10000 {
            assert!(
                bf.maybe_contains(hash_value(&i)),
                "False negative for {}",
                i
            );
        }
    }

    #[test]
    fn empty_filter_rejects_many() {
        let bf = BloomFilter::new(100, 0.01);
        for i in 0..100 {
            assert!(!bf.maybe_contains(hash_value(&i)));
        }
    }
}
