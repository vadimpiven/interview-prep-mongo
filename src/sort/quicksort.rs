// Quicksort — Unstable, O(n log n) average, O(n^2) worst case, O(1) extra space
//
// MongoDB uses std::sort (introsort: quicksort + heapsort fallback + insertion sort
// for small partitions) for all in-memory sorting. It does NOT implement quicksort
// from scratch. The in-memory sort is always unstable — stability comes from the
// merge phase's sourceId tiebreaker.
//
// When quicksort beats merge sort:
//   - In-memory sorting (cache-friendly in-place partitioning)
//   - Average case faster constant factor than merge sort
//   - O(log n) stack space vs O(n) auxiliary space for merge sort
//
// When quicksort is impractical (HR's words):
//   - Stability required -> use merge sort
//   - Worst-case guarantee needed -> quicksort degrades to O(n^2) on sorted input
//   - External sort -> merge sort's merge phase works on sequential disk runs
//
// This implementation uses Lomuto partition (simpler to write in interview).
// Hoare partition is faster in practice but harder to get right.
//
// Algorithm:
//   1. Pick pivot (last element in Lomuto scheme)
//   2. Partition: elements < pivot go left, >= pivot go right
//   3. Place pivot at boundary
//   4. Recurse on left and right partitions

// --- Helper (defined before caller, C-style) ---

/// Lomuto partition: pick last element as pivot.
/// Walks array once, swapping elements smaller than pivot to the front.
/// Returns final pivot position.
///
/// Invariant: `arr[0..i]` < pivot, `arr[i..j]` >= pivot, `arr[pivot_idx]` = pivot
fn partition<T: Ord>(arr: &mut [T]) -> usize {
    let len = arr.len();
    let pivot_idx = len - 1;
    let mut i = 0;

    for j in 0..pivot_idx {
        if arr[j] < arr[pivot_idx] {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, pivot_idx);
    i
}

// --- Public API ---

/// Quicksort -- in-place, unstable, O(n log n) average.
pub fn quicksort<T: Ord>(arr: &mut [T]) {
    if arr.len() <= 1 {
        return;
    }
    let pivot_idx = partition(arr);
    quicksort(&mut arr[..pivot_idx]);
    quicksort(&mut arr[pivot_idx + 1..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn empty() {
        let mut arr: Vec<i32> = vec![];
        quicksort(&mut arr);
        assert!(arr.is_empty());
    }

    // ONE
    #[test]
    fn single() {
        let mut arr = vec![42];
        quicksort(&mut arr);
        assert_eq!(arr, vec![42]);
    }

    // MANY
    #[test]
    fn basic() {
        let mut arr = vec![5, 3, 1, 4, 2];
        quicksort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: already sorted (worst case for Lomuto with last-element pivot)
    #[test]
    fn already_sorted() {
        let mut arr = vec![1, 2, 3, 4, 5];
        quicksort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: reverse sorted
    #[test]
    fn reverse() {
        let mut arr = vec![5, 4, 3, 2, 1];
        quicksort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: duplicates
    #[test]
    fn duplicates() {
        let mut arr = vec![3, 1, 2, 1, 3, 2];
        quicksort(&mut arr);
        assert_eq!(arr, vec![1, 1, 2, 2, 3, 3]);
    }

    // EDGE: all equal (tests partition doesn't infinite-loop)
    #[test]
    fn all_same() {
        let mut arr = vec![7, 7, 7, 7];
        quicksort(&mut arr);
        assert_eq!(arr, vec![7, 7, 7, 7]);
    }
}
