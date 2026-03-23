// MongoDB QE Technical Screen — Practice Implementations
//
// Each module is a self-contained answer to a likely interview problem.
// Run all tests: cargo test
// Run one module: cargo test hash_map
//
// Modules ordered from most to least likely to encounter.

// -- General CS fundamentals (expected in any coding interview) --

/// Open-addressing hash map built on a Vec (no std::collections).
/// HR explicitly asks: "implement a hash table using only arrays."
/// MongoDB uses absl::flat_hash_map (same concept: flat, cache-friendly, open addressing).
pub mod hash_map;

/// Merge sort and quicksort — from-scratch implementations.
/// "Know the details of at least one n*log(n) sorting algorithm, preferably two."
pub mod sorting;

/// BST with traversals + AVL tree (self-balancing BST with rotations).
/// "Know about trees; basic tree construction, traversal and manipulation algorithms.
/// Be familiar with at least one type of balanced binary tree."
pub mod trees;

// -- MongoDB-specific data structures and algorithms --

/// LRU cache, SBE stages, k-way merge, external sort, bloom filter.
/// Implementations modeled after MongoDB's query execution engine internals.
pub mod mongo;
