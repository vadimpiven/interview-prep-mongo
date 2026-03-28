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

/// Heapsort -- in-place, unstable, O(n log n) worst case.
pub fn heapsort<T: Ord>(arr: &mut [T]) {
    heapsort_by(arr, T::cmp);
}

/// Heapsort with custom comparator.
pub fn heapsort_by<T, F: Fn(&T, &T) -> std::cmp::Ordering>(arr: &mut [T], cmp: F) {
    let mut heap = MaxHeap { arr, cmp };
    heap.build();
    heap.sort();
}

/// Max-heap backed by a mutable slice. Tracks the live heap region
/// (`arr[..len]`) separately from the sorted tail (`arr[len..]`).
struct MaxHeap<'a, T, F> {
    arr: &'a mut [T],
    cmp: F,
}

impl<T, F: Fn(&T, &T) -> std::cmp::Ordering> MaxHeap<'_, T, F> {
    /// Build a max-heap in place. Start from the last non-leaf and sift down.
    /// Last non-leaf = parent of last element = (len - 2) / 2.
    fn build(&mut self) {
        let len = self.arr.len();
        if len <= 1 {
            return;
        }
        for i in (0..=(len - 2) / 2).rev() {
            self.sift_down(i, len);
        }
    }

    /// Extract max repeatedly: swap root with last unsorted element,
    /// shrink heap by one, restore heap property.
    fn sort(&mut self) {
        for end in (1..self.arr.len()).rev() {
            self.arr.swap(0, end);
            self.sift_down(0, end);
        }
    }

    /// Sift element at `pos` down to restore max-heap property within `arr[..heap_len]`.
    ///
    /// Invariant: both children of `pos` are valid max-heaps; only `pos` itself
    /// may violate the heap property.
    fn sift_down(&mut self, mut pos: usize, heap_len: usize) {
        loop {
            let left = 2 * pos + 1;
            if left >= heap_len {
                break;
            }
            // Pick the larger child.
            let right = left + 1;
            let child = if right < heap_len && (self.cmp)(&self.arr[right], &self.arr[left]).is_gt()
            {
                right
            } else {
                left
            };
            // If parent is already >= largest child, heap property holds.
            if (self.cmp)(&self.arr[pos], &self.arr[child]).is_ge() {
                break;
            }
            self.arr.swap(pos, child);
            pos = child;
        }
    }
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
