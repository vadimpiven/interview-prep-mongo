// External Sort — Memory-bounded sort with spilling
//
// MongoDB's Sorter (src/mongo/db/sorter/) implements this exact algorithm:
//   Phase 1: Sort chunks in memory until memory limit reached
//            -> write each sorted chunk as a "run" to a temp Vec (simulating disk)
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

use super::k_way_merge::MergeIterator;
use std::collections::BinaryHeap;
use std::marker::PhantomData;

/// External sorter that spills to "disk" (simulated as `Vec<Vec<T>>`)
/// when in-memory buffer exceeds `max_memory_items`.
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
    ///
    /// # Panics
    ///
    /// Panics if `max_memory_items` is 0.
    #[must_use]
    pub fn new(max_memory_items: usize) -> Self {
        assert!(max_memory_items > 0, "buffer capacity must be at least 1");
        Self {
            max_memory_items,
            _phantom: PhantomData,
        }
    }

    /// Sort the input. Returns sorted `Vec` + stats.
    ///
    /// Phase 1: accumulate into buffer, sort and "spill" when buffer full.
    /// Phase 2: K-way merge all sorted runs via `MergeIterator`.
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
        let iters: Vec<_> = runs.into_iter().map(IntoIterator::into_iter).collect();
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
/// O(N log K) time, O(K) space -- much better than full sort for small K.
///
/// O(n log k) time, O(k) space. `MongoDB`'s `TopKSorter` uses the same approach.
#[must_use]
pub fn top_k<T: Ord>(input: impl IntoIterator<Item = T>, k: usize) -> Vec<T> {
    // Max-heap of size K: root is the largest of the K smallest seen so far.
    // If new element < root, pop root and push new element.
    let mut heap = BinaryHeap::with_capacity(k + 1);

    for item in input {
        if heap.len() < k {
            heap.push(item);
        } else if heap.peek().is_some_and(|max| item < *max) {
            heap.pop();
            heap.push(item);
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

    // ZERO
    #[test]
    fn sort_empty() {
        let sorter = ExternalSorter::<i32>::new(10);
        let (result, stats) = sorter.sort(vec![]);
        assert!(result.is_empty());
        assert_eq!(stats.total_items, 0);
    }

    // ONE
    #[test]
    fn sort_single() {
        let sorter = ExternalSorter::new(10);
        let (result, _) = sorter.sort(vec![42]);
        assert_eq!(result, vec![42]);
    }

    // MANY — fits in memory (no spill)
    #[test]
    fn sort_in_memory() {
        let sorter = ExternalSorter::new(100);
        let (result, stats) = sorter.sort(vec![5, 3, 1, 4, 2]);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
        assert!(!stats.spilled);
    }

    // MANY — triggers spilling
    #[test]
    fn sort_with_spill() {
        let sorter = ExternalSorter::new(3);
        let (result, stats) = sorter.sort(vec![9, 7, 5, 3, 1, 8, 6, 4, 2]);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert!(stats.spilled);
    }

    // top_k: MANY
    #[test]
    fn top_k_basic() {
        assert_eq!(top_k(vec![5, 3, 1, 4, 2], 3), vec![1, 2, 3]);
    }

    // top_k EDGE: k > input size
    #[test]
    fn top_k_exceeds_input() {
        assert_eq!(top_k(vec![3, 1, 2], 10), vec![1, 2, 3]);
    }
}
