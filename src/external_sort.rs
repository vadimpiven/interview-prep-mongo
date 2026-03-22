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
// In real MongoDB: runs are serialized to temp files with snappy compression.
// Here we simulate with Vec<Vec<T>> for testability.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::marker::PhantomData;

/// External sorter that spills to "disk" (simulated as Vec<Vec<T>>)
/// when in-memory buffer exceeds max_memory_items.
pub struct ExternalSorter<T: Ord + Clone> {
    max_memory_items: usize,
    _phantom: PhantomData<T>,
}

/// Statistics returned after sorting, for discussion in interview.
#[derive(Debug)]
pub struct SortStats {
    pub total_items: usize,
    pub num_runs: usize, // 1 = entirely in-memory, >1 = spilled
    pub spilled: bool,
}

impl<T: Ord + Clone> ExternalSorter<T> {
    pub fn new(max_memory_items: usize) -> Self {
        Self {
            max_memory_items,
            _phantom: PhantomData,
        }
    }

    /// Sort the input. Returns sorted Vec + stats.
    ///
    /// Phase 1: accumulate into buffer, sort and "spill" when buffer full.
    /// Phase 2: K-way merge all sorted runs.
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

        // Phase 2: K-way merge
        let result = self.k_way_merge(runs);

        (
            result,
            SortStats {
                total_items,
                num_runs,
                spilled,
            },
        )
    }

    /// Merge K sorted runs using a min-heap.
    /// Same algorithm as MergeIterator in sorter_template_defs.h.
    fn k_way_merge(&self, runs: Vec<Vec<T>>) -> Vec<T> {
        if runs.len() == 1 {
            return runs.into_iter().next().unwrap();
        }

        let mut heap: BinaryHeap<RunEntry<T>> = BinaryHeap::new();

        // Initialize heap with first element from each run
        for (run_id, run) in runs.into_iter().enumerate() {
            let mut remaining: VecDeque<T> = run.into();
            if let Some(val) = remaining.pop_front() {
                heap.push(RunEntry {
                    value: val,
                    run_id,
                    remaining,
                });
            }
        }

        let mut result = Vec::new();

        while let Some(mut entry) = heap.pop() {
            let val = if let Some(next_val) = entry.remaining.pop_front() {
                let output = std::mem::replace(&mut entry.value, next_val);
                heap.push(entry);
                output
            } else {
                entry.value // last element from this run
            };
            result.push(val);
        }

        result
    }
}

/// Heap entry: wraps a value with its source run for the K-way merge.
/// Ord is reversed for min-heap (BinaryHeap is max-heap).
struct RunEntry<T: Ord> {
    value: T,
    run_id: usize,
    remaining: VecDeque<T>,
}

impl<T: Ord> Ord for RunEntry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .value
            .cmp(&self.value) // reversed for min-heap
            .then(other.run_id.cmp(&self.run_id)) // stable
    }
}

impl<T: Ord> PartialOrd for RunEntry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> PartialEq for RunEntry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.run_id == other.run_id
    }
}

impl<T: Ord> Eq for RunEntry<T> {}

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

    // -- External sort tests --

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
    fn sort_empty() {
        let sorter = ExternalSorter::<i32>::new(10);
        let (result, stats) = sorter.sort(vec![]);
        assert!(result.is_empty());
        assert_eq!(stats.total_items, 0);
    }

    #[test]
    fn sort_single_element() {
        let sorter = ExternalSorter::new(10);
        let (result, _) = sorter.sort(vec![42]);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn sort_already_sorted() {
        let sorter = ExternalSorter::new(3);
        let (result, _) = sorter.sort(vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn sort_reverse_sorted() {
        let sorter = ExternalSorter::new(3);
        let (result, _) = sorter.sort(vec![6, 5, 4, 3, 2, 1]);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn sort_duplicates() {
        let sorter = ExternalSorter::new(3);
        let (result, _) = sorter.sort(vec![3, 1, 2, 1, 3, 2]);
        assert_eq!(result, vec![1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn sort_large_input() {
        let sorter = ExternalSorter::new(100);
        let input: Vec<i32> = (0..1000).rev().collect();
        let (result, stats) = sorter.sort(input);
        let expected: Vec<i32> = (0..1000).collect();
        assert_eq!(result, expected);
        assert!(stats.spilled);
    }

    // -- TopK tests --

    #[test]
    fn top_k_basic() {
        let result = top_k(vec![5, 3, 1, 4, 2], 3);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn top_k_larger_than_input() {
        let result = top_k(vec![3, 1, 2], 10);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn top_k_one() {
        let result = top_k(vec![5, 3, 1, 4, 2], 1);
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn top_k_zero() {
        let result = top_k(vec![5, 3, 1], 0);
        assert!(result.is_empty());
    }

    #[test]
    fn top_k_empty_input() {
        let result = top_k(Vec::<i32>::new(), 5);
        assert!(result.is_empty());
    }

    #[test]
    fn top_k_with_duplicates() {
        let result = top_k(vec![3, 1, 2, 1, 3, 2], 4);
        assert_eq!(result, vec![1, 1, 2, 2]);
    }

    // -- External sort edge cases --

    #[test]
    fn sort_exact_buffer_boundary() {
        // Input size is exact multiple of max_memory_items
        let sorter = ExternalSorter::new(3);
        let (result, stats) = sorter.sort(vec![6, 5, 4, 3, 2, 1]); // 6 items, buffer=3
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(stats.num_runs, 2);
        assert!(stats.spilled);
    }

    #[test]
    fn sort_buffer_size_one() {
        // Degenerate: every element is its own run
        let sorter = ExternalSorter::new(1);
        let (result, stats) = sorter.sort(vec![3, 1, 2]);
        assert_eq!(result, vec![1, 2, 3]);
        assert_eq!(stats.num_runs, 3);
    }

    #[test]
    fn sort_all_same_values() {
        let sorter = ExternalSorter::new(2);
        let (result, _) = sorter.sort(vec![5, 5, 5, 5, 5]);
        assert_eq!(result, vec![5, 5, 5, 5, 5]);
    }

    #[test]
    fn sort_two_elements_reversed() {
        let sorter = ExternalSorter::new(10);
        let (result, stats) = sorter.sort(vec![2, 1]);
        assert_eq!(result, vec![1, 2]);
        assert!(!stats.spilled);
    }

    #[test]
    fn sort_stats_correct_item_count() {
        let sorter = ExternalSorter::new(5);
        let (_, stats) = sorter.sort(vec![10, 20, 30, 40, 50, 60, 70]);
        assert_eq!(stats.total_items, 7);
        assert_eq!(stats.num_runs, 2); // 5 + 2
    }

    // -- TopK edge cases --

    #[test]
    fn top_k_equals_input_length() {
        let result = top_k(vec![5, 3, 1, 4, 2], 5);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn top_k_all_same() {
        let result = top_k(vec![7, 7, 7, 7], 2);
        assert_eq!(result, vec![7, 7]);
    }

    #[test]
    fn top_k_single_element_input() {
        let result = top_k(vec![42], 1);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn top_k_already_sorted() {
        let result = top_k(vec![1, 2, 3, 4, 5], 3);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn top_k_reverse_sorted() {
        let result = top_k(vec![5, 4, 3, 2, 1], 3);
        assert_eq!(result, vec![1, 2, 3]);
    }
}
