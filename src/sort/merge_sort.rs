// Merge Sort — Stable, O(n log n) worst case, O(n) extra space
//
// HR says: "Merge sort can be highly useful in situations where quicksort
// is impractical, so take a look at it."
//
// MongoDB relevance:
//   - MongoDB does NOT implement merge sort from scratch for in-memory data.
//     It uses std::sort (introsort/pdqsort) — unstable, O(n log n) worst case.
//   - Stability is achieved at the MERGE level, not the sort level:
//     MergeIterator breaks ties by sourceId (lower = earlier run = original order).
//   - The merge phase (this module's merge function) is what MongoDB actually
//     implements: k-way merge of sorted runs via a min-heap (see k_way_merge module).
//   - MongoDB's comparator pattern: 3-way comparison returning int
//     (negative/zero/positive), wrapped for std::sort's bool requirement.
//
// When merge sort beats quicksort:
//   - Stability required (MongoDB's $sort guarantee — via merge, not sort)
//   - External sort (merge phase works on sequential runs from disk)
//   - Linked lists (merge sort is O(1) extra space on linked lists)
//   - Worst-case guarantee needed (quicksort is O(n^2) worst case)
//
// Algorithm (bottom-up, iterative):
//   1. Start with width = 1 (each element is a sorted run)
//   2. Merge adjacent pairs of runs of size `width` into runs of size `2*width`
//   3. Double `width` and repeat until the whole array is one sorted run
//   Stability: on ties, pick from left half -> preserves original order

use std::cmp::Ordering;

/// Merge sort -- stable, O(n log n) time, O(n) space.
pub fn merge_sort<T: Ord + Clone>(arr: &mut [T]) {
    merge_sort_by(arr, T::cmp);
}

/// Merge sort with custom comparator. O(n log n) time, O(n) space.
///
/// Bottom-up (iterative): merge runs of width 1, 2, 4, 8, ...
/// avoiding recursion and stack overflow on large inputs.
pub fn merge_sort_by<T: Clone, F: Fn(&T, &T) -> Ordering>(arr: &mut [T], cmp: F) {
    let len = arr.len();
    if len <= 1 {
        return;
    }

    let mut buf = arr.to_vec();
    let mut width = 1;

    while width < len {
        for lo in (0..len).step_by(2 * width) {
            let mid = (lo + width).min(len);
            let hi = (lo + 2 * width).min(len);

            // Merge arr[lo..mid] and arr[mid..hi] into buf[lo..hi].
            let (mut i, mut j, mut k) = (lo, mid, lo);
            while i < mid && j < hi {
                // <= for stability: prefer left element on ties
                if cmp(&arr[i], &arr[j]).is_le() {
                    buf[k] = arr[i].clone();
                    i += 1;
                } else {
                    buf[k] = arr[j].clone();
                    j += 1;
                }
                k += 1;
            }
            while i < mid {
                buf[k] = arr[i].clone();
                i += 1;
                k += 1;
            }
            while j < hi {
                buf[k] = arr[j].clone();
                j += 1;
                k += 1;
            }
        }
        arr.clone_from_slice(&buf);
        width *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn empty() {
        let mut arr: Vec<i32> = vec![];
        merge_sort(&mut arr);
        assert!(arr.is_empty());
    }

    // ONE
    #[test]
    fn single() {
        let mut arr = vec![42];
        merge_sort(&mut arr);
        assert_eq!(arr, vec![42]);
    }

    // MANY
    #[test]
    fn basic() {
        let mut arr = vec![5, 3, 1, 4, 2];
        merge_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: already sorted input
    #[test]
    fn already_sorted() {
        let mut arr = vec![1, 2, 3, 4, 5];
        merge_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: reverse sorted
    #[test]
    fn reverse() {
        let mut arr = vec![5, 4, 3, 2, 1];
        merge_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: duplicates
    #[test]
    fn duplicates() {
        let mut arr = vec![3, 1, 2, 1, 3, 2];
        merge_sort(&mut arr);
        assert_eq!(arr, vec![1, 1, 2, 2, 3, 3]);
    }

    // EDGE: stability — equal elements preserve original order
    #[test]
    fn stable() {
        let mut arr = vec![(3, 0), (1, 1), (3, 2), (1, 3)];
        merge_sort_by(&mut arr, |a, b| a.0.cmp(&b.0));
        assert_eq!(arr, vec![(1, 1), (1, 3), (3, 0), (3, 2)]);
    }
}
