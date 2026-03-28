// Sorting and merging algorithms.
// HR: "Know how to sort. Know the details of at least one n*log(n) algorithm, preferably two."

/// Merge sort -- stable, O(n log n), O(n) space. Includes `merge_sort_by` for custom comparators.
/// `MongoDB`: merge phase used in external sorter's k-way merge of spilled runs.
pub mod merge_sort;

/// Quicksort -- unstable, O(n log n) average, in-place. Lomuto partition scheme.
/// `MongoDB`: uses `std::sort` (introsort) for in-memory chunks -- same family.
pub mod quicksort;

/// Heapsort -- unstable, O(n log n) worst case, in-place. Max-heap + extract.
/// Introsort falls back to this when quicksort recurses too deep.
pub mod heapsort;

/// Insertion sort -- stable, O(n^2), but fastest for small arrays (< ~16 elements).
/// Introsort switches to this for small partitions due to low overhead.
pub mod insertion_sort;

/// K-way merge of sorted iterators using `BinaryHeap`.
/// HR: "handling obscenely large amounts of data."
/// `MongoDB`: `MergeIterator` in `sorter_template_defs.h`, change stream shard merging.
pub mod k_way_merge;

/// External sort -- memory-bounded sort with spilling + top-K.
/// HR: "handling obscenely large amounts of data" + sorting.
/// `MongoDB`: Sorter (`src/mongo/db/sorter/`) -- sort chunks -> spill runs -> k-way merge.
pub mod external_sort;
