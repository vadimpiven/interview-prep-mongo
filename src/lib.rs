// MongoDB QE Technical Screen — Practice Implementations
//
// Each module is a self-contained answer to a likely interview problem.
// Run all tests:       cargo test
// Run one group:       cargo test hash_maps
// Run one module:      cargo test hash_map
// Run one test:        cargo test hash_map::tests::insert_and_get
//
// +-------------------------------------------------------------+
// |  hash_maps/         -- hash table, bloom filter, LRU cache   |
// |  sort/              -- merge sort, quicksort, k-way merge,   |
// |                       external sort with spilling             |
// |  trees/             -- binary tree + BST, trie, n-ary        |
// |                       expression tree (MongoDB `MatchExpression`) |
// |  mongo/             -- SBE pull-based stages (filter, join, agg) |
// +-------------------------------------------------------------+

/// Hash-based data structures: hash map from arrays, bloom filter, LRU cache.
/// HR: "Arguably the single most important data structure known to mankind."
pub mod hash_maps;

/// Sorting and merging: merge sort, quicksort, k-way merge, external sort + top-K.
/// HR: "Know the details of at least one `n*log(n)` algorithm, preferably two."
pub mod sort;

/// Trees: binary tree traversal + BST, trie, n-ary expression tree with optimize/evaluate.
/// HR: "Know about trees; basic tree construction, traversal and manipulation."
pub mod trees;

/// MongoDB-specific: pull-based SBE stages (filter, limit/skip, hash agg, hash join).
/// HR: "Implement system routines, distill large data sets, transform one data set to another."
pub mod mongo;
