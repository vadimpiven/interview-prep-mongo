// External Sort — Memory-bounded sort with spilling
//
// MongoDB's Sorter (src/mongo/db/sorter/) implements this exact algorithm:
//   Phase 1: Sort chunks in memory until memory limit reached
//            → write each sorted chunk as a "run" to a temp Vec (simulating disk)
//   Phase 2: K-way merge all runs using a min-heap
//
// Variants in MongoDB:
//   NoLimitSorter  — full sort, spills when memory exceeded
//   TopKSorter     — maintains heap of K best, O(N log K)
//   LimitOneSorter — optimized for limit=1
//   BoundedSorter  — for "almost sorted" input with known bounds
//
// Complexity:
//   Memory-only: O(N log N) time, O(N) space
//   With spilling: O(N log N) time, O(M) space where M = memory limit
//   K-way merge: O(N log K) where K = number of runs
//
// Note: this simulates disk spilling for algorithmic correctness but does not
// actually reduce peak memory — all runs remain in memory as Vec<Vec<T>>.
// In real MongoDB, runs are serialized to temp files with snappy compression.

use crate::k_way_merge::MergeIterator;
use std::collections::BinaryHeap;
use std::marker::PhantomData;

/// External sorter that spills to "disk" (simulated as Vec<Vec<T>>)
/// when in-memory buffer exceeds max_memory_items.
pub struct ExternalSorter<T> {
    max_memory_items: usize,
    _phantom: PhantomData<T>,
}

/// Statistics returned after sorting, for discussion in interview.
#[derive(Debug, PartialEq, Eq)]
pub struct SortStats {
    pub total_items: usize,
    pub num_runs: usize, // 1 = entirely in-memory, >1 = spilled
    pub spilled: bool,
}

impl<T: Ord> ExternalSorter<T> {
    /// O(1).
    pub fn new(max_memory_items: usize) -> Self {
        assert!(max_memory_items > 0, "buffer capacity must be at least 1");
        Self {
            max_memory_items,
            _phantom: PhantomData,
        }
    }

    /// Sort the input. Returns sorted Vec + stats.
    ///
    /// Phase 1: accumulate into buffer, sort and "spill" when buffer full.
    /// Phase 2: K-way merge all sorted runs via MergeIterator.
    ///
    /// O(N log N) time, O(N) space. With K runs: merge phase is O(N log K).
    pub fn sort(&self, input: impl IntoIterator<Item = T>) -> (Vec<T>, SortStats) {
        let mut runs: Vec<Vec<T>> = Vec::new();
        let mut buffer: Vec<T> = Vec::new();
        let mut total_items = 0;

        // Phase 1: partition input into sorted runs
        for item in input {
            buffer.push(item);
            total_items += 1;

            if buffer.len() >= self.max_memory_items {
                buffer.sort();
                runs.push(std::mem::take(&mut buffer));
            }
        }

        // Flush remaining buffer
        if !buffer.is_empty() {
            buffer.sort();
            runs.push(buffer);
        }

        let num_runs = runs.len();
        let spilled = num_runs > 1;

        if runs.is_empty() {
            return (
                Vec::new(),
                SortStats {
                    total_items: 0,
                    num_runs: 0,
                    spilled: false,
                },
            );
        }

        // Phase 2: K-way merge using the shared MergeIterator
        let iters: Vec<_> = runs.into_iter().map(|r| r.into_iter()).collect();
        let result = MergeIterator::new(iters).collect();

        (
            result,
            SortStats {
                total_items,
                num_runs,
                spilled,
            },
        )
    }
}

// ---------------------------------------------------------------------------
// TopK — Bounded heap for ORDER BY + LIMIT K
// ---------------------------------------------------------------------------

/// Returns the K smallest elements from input in sorted order.
/// O(N log K) time, O(K) space — much better than full sort for small K.
///
/// MongoDB's TopKSorter uses the same approach.
pub fn top_k<T: Ord>(input: impl IntoIterator<Item = T>, k: usize) -> Vec<T> {
    // Max-heap of size K: root is the largest of the K smallest seen so far.
    // If new element < root, pop root and push new element.
    let mut heap = BinaryHeap::with_capacity(k + 1);

    for item in input {
        if heap.len() < k {
            heap.push(item);
        } else if let Some(max) = heap.peek() {
            if item < *max {
                heap.pop();
                heap.push(item);
            }
        }
    }

    // Drain heap into sorted order (heap gives largest first, so reverse)
    let mut result: Vec<T> = heap.into_sorted_vec();
    result.truncate(k);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_fits_in_memory() {
        let sorter = ExternalSorter::new(100);
        let (result, stats) = sorter.sort(vec![5, 3, 1, 4, 2]);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
        assert!(!stats.spilled);
        assert_eq!(stats.num_runs, 1);
    }

    #[test]
    fn sort_spills_to_disk() {
        let sorter = ExternalSorter::new(3); // only 3 items in memory
        let (result, stats) = sorter.sort(vec![9, 7, 5, 3, 1, 8, 6, 4, 2]);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert!(stats.spilled);
        assert!(stats.num_runs > 1);
    }

    #[test]
    fn sort_duplicates() {
        let sorter = ExternalSorter::new(3);
        let (result, _) = sorter.sort(vec![3, 1, 2, 1, 3, 2]);
        assert_eq!(result, vec![1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn top_k_basic() {
        let result = top_k(vec![5, 3, 1, 4, 2], 3);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn top_k_with_duplicates() {
        let result = top_k(vec![3, 1, 2, 1, 3, 2], 4);
        assert_eq!(result, vec![1, 1, 2, 2]);
    }
}
