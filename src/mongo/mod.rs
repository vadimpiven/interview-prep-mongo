// MongoDB-specific data structures and algorithms.
//
// These modules implement patterns found in MongoDB's query execution engine
// (SBE), plan cache, sorter, and hash join stages.

/// LRU cache: HashMap + index-based doubly-linked list for O(1) get/insert.
/// MongoDB's plan cache (lru_key_value.h) uses this exact pattern.
pub mod lru_cache;

/// Pull-based iterator stages: the core SBE execution model.
/// Stage trait with open/get_next/close lifecycle.
/// Includes: FilterStage, LimitSkipStage, HashAggStage, HashJoinStage.
pub mod stages;

/// K-way merge of sorted iterators using a BinaryHeap.
/// Used in MongoDB's external sort merge phase (src/mongo/db/sorter/).
/// Also relevant to change stream event merging across shards.
pub mod k_way_merge;

/// External sort with memory-bounded spilling.
/// MongoDB's Sorter (src/mongo/db/sorter/) does exactly this:
/// sort chunks in memory → spill sorted runs → k-way merge.
pub mod external_sort;

/// Bloom filter with cache-line-aligned blocks and 8 hash functions.
/// Used in MongoDB's HybridHashJoinStage to skip non-matching probe rows.
pub mod bloom_filter;
