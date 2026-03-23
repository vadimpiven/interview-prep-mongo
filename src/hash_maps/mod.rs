// Hash-based data structures.
// HR: "Arguably the single most important data structure known to mankind."

/// Open-addressing hash map built on a `Vec` (no `std::collections`).
/// HR: "Be able to implement one using only arrays in about the space of one interview."
/// `MongoDB`: `absl::flat_hash_map` (same concept -- flat, cache-friendly, open addressing).
pub mod hash_map;

/// Bloom filter -- cache-line-aligned blocks, 8 hash functions, no false negatives.
/// HR: "hashing" (advanced application).
/// `MongoDB`: `HybridHashJoinStage` uses bloom filter to skip non-matching probe rows.
pub mod bloom_filter;

/// LRU cache -- `HashMap` + `VecDeque` for O(1) amortized get/put.
/// HR: "construct data structures" (hash table application).
/// `MongoDB`: plan cache (`lru_key_value.h`) uses this exact pattern.
pub mod lru_cache;
