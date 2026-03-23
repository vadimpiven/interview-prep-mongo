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
// Algorithm:
//   1. Split array in half
//   2. Recursively sort each half
//   3. Merge two sorted halves (linear scan, pick smaller element)
//   Stability: on ties, pick from left half -> preserves original order

// --- Helpers (defined before callers, C-style) ---

/// Merge two sorted slices into a single sorted `Vec`.
/// This is the same operation as `MongoDB`'s `MergeIterator` for 2 runs.
fn merge<T: Ord + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    let mut result = Vec::with_capacity(left.len() + right.len());
    let (mut i, mut j) = (0, 0);

    while i < left.len() && j < right.len() {
        // <= for stability: prefer left element on ties
        if left[i] <= right[j] {
            result.push(left[i].clone());
            i += 1;
        } else {
            result.push(right[j].clone());
            j += 1;
        }
    }

    result.extend_from_slice(&left[i..]);
    result.extend_from_slice(&right[j..]);
    result
}

fn merge_by<T: Clone, F: Fn(&T, &T) -> std::cmp::Ordering>(
    left: &[T],
    right: &[T],
    cmp: F,
) -> Vec<T> {
    let mut result = Vec::with_capacity(left.len() + right.len());
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if cmp(&left[i], &right[j]).is_le() {
            result.push(left[i].clone());
            i += 1;
        } else {
            result.push(right[j].clone());
            j += 1;
        }
    }
    result.extend_from_slice(&left[i..]);
    result.extend_from_slice(&right[j..]);
    result
}

// --- Public API ---

/// Merge sort -- stable, O(n log n) time, O(n) space.
pub fn merge_sort<T: Ord + Clone>(arr: &mut [T]) {
    let len = arr.len();
    if len <= 1 {
        return;
    }
    let mid = len / 2;

    merge_sort(&mut arr[..mid]);
    merge_sort(&mut arr[mid..]);

    let merged = merge(&arr[..mid], &arr[mid..]);
    arr.clone_from_slice(&merged);
}

/// Merge sort with custom comparator. O(n log n) time, O(n) space.
/// Needed for stability testing and for sorting by a key function.
pub fn merge_sort_by<T: Clone, F: Fn(&T, &T) -> std::cmp::Ordering + Copy>(arr: &mut [T], cmp: F) {
    let len = arr.len();
    if len <= 1 {
        return;
    }
    let mid = len / 2;
    merge_sort_by(&mut arr[..mid], cmp);
    merge_sort_by(&mut arr[mid..], cmp);
    let merged = merge_by(&arr[..mid], &arr[mid..], cmp);
    arr.clone_from_slice(&merged);
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
