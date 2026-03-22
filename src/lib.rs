// MongoDB QE Technical Screen — Practice Implementations
//
// Each module is a self-contained answer to a likely interview problem.
// Run all tests: cargo test
// Run one module: cargo test hash_map
//
// Modules ordered by likelihood (based on team's recent work and HR topic list):

/// Open-addressing hash map built on a Vec (no std::collections).
/// HR explicitly asks: "implement a hash table using only arrays."
/// MongoDB uses absl::flat_hash_map (same concept: flat, cache-friendly, open addressing).
pub mod hash_map;

/// K-way merge of sorted iterators using a BinaryHeap.
/// Used in MongoDB's external sort merge phase (src/mongo/db/sorter/).
/// Also relevant to change stream event merging across shards.
pub mod k_way_merge;

/// LRU cache: VecDeque with linear scan for O(n) get/insert.
/// MongoDB's plan cache (lru_key_value.h) uses this exact pattern.
pub mod lru_cache;

/// Bloom filter with cache-line-aligned blocks and 8 hash functions.
/// Used in MongoDB's HybridHashJoinStage to skip non-matching probe rows.
pub mod bloom_filter;

/// Pull-based iterator stages: the core SBE execution model.
/// Stage trait with open/get_next/close lifecycle.
/// Includes: FilterStage, LimitSkipStage, HashAggStage, HashJoinStage.
pub mod stages;

/// External sort with memory-bounded spilling.
/// MongoDB's Sorter (src/mongo/db/sorter/) does exactly this:
/// sort chunks in memory → spill sorted runs → k-way merge.
pub mod external_sort;
