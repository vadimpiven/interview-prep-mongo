// Heapsort — Unstable, O(n log n) worst case, O(1) extra space
//
// Heapsort guarantees O(n log n) regardless of input — no degenerate cases.
// This is why introsort falls back to it when quicksort recurses too deep.
//
// Trade-offs vs quicksort:
//   - Worse cache locality (heap jumps around the array: parent at i,
//     children at 2i+1 and 2i+2 — far apart for large arrays)
//   - ~2-3x slower in practice despite same O(n log n) asymptotic bound
//   - But: guaranteed O(n log n), no stack usage, truly in-place
//
// Trade-offs vs merge sort:
//   - O(1) space vs O(n) — no allocation needed
//   - Unstable — cannot preserve original order of equal elements
//   - Cannot be used for external sort (no merge phase)
//
// Algorithm:
//   1. Build a max-heap in-place (heapify from bottom up)
//   2. Repeatedly swap root (max) to the end, shrink heap, sift down
//   After step 2, array is sorted in ascending order.

use std::cmp::Ordering;

use super::heap::Heap;

/// Heapsort -- in-place, unstable, O(n log n) worst case.
pub fn heapsort<T: Ord + Clone>(arr: &mut [T]) {
    heapsort_by(arr, T::cmp);
}

/// Heapsort with custom comparator.
///
/// Builds a max-heap from the data, then extracts elements in sorted order.
/// Uses the shared `Heap` struct (same one k-way merge uses as a min-heap).
pub fn heapsort_by<T: Clone, F: Fn(&T, &T) -> Ordering>(arr: &mut [T], cmp: F) {
    let heap = Heap::from_vec(arr.to_vec(), cmp);
    let sorted = heap.into_sorted_vec();
    arr.clone_from_slice(&sorted);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn empty() {
        let mut arr: Vec<i32> = vec![];
        heapsort(&mut arr);
        assert!(arr.is_empty());
    }

    // ONE
    #[test]
    fn single() {
        let mut arr = vec![42];
        heapsort(&mut arr);
        assert_eq!(arr, vec![42]);
    }

    // TWO
    #[test]
    fn two_elements() {
        let mut arr = vec![2, 1];
        heapsort(&mut arr);
        assert_eq!(arr, vec![1, 2]);
    }

    // MANY
    #[test]
    fn basic() {
        let mut arr = vec![5, 3, 1, 4, 2];
        heapsort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: already sorted
    #[test]
    fn already_sorted() {
        let mut arr = vec![1, 2, 3, 4, 5];
        heapsort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: reverse sorted
    #[test]
    fn reverse() {
        let mut arr = vec![5, 4, 3, 2, 1];
        heapsort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: duplicates
    #[test]
    fn duplicates() {
        let mut arr = vec![3, 1, 2, 1, 3, 2];
        heapsort(&mut arr);
        assert_eq!(arr, vec![1, 1, 2, 2, 3, 3]);
    }

    // EDGE: all equal
    #[test]
    fn all_same() {
        let mut arr = vec![7, 7, 7, 7];
        heapsort(&mut arr);
        assert_eq!(arr, vec![7, 7, 7, 7]);
    }
}
