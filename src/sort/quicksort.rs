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

/// Quicksort -- in-place, unstable, O(n log n) average.
///
/// Iterative (explicit stack) to avoid call-stack overflow on large or
/// degenerate inputs. Uses Lomuto partition: pick last element as pivot,
/// walk the sub-array once swapping elements smaller than pivot to the front.
pub fn quicksort<T: Ord>(arr: &mut [T]) {
    let mut stack: Vec<(usize, usize)> = vec![(0, arr.len())];

    while let Some((lo, hi)) = stack.pop() {
        if hi - lo <= 1 {
            continue;
        }

        // Lomuto partition on arr[lo..hi].
        // Invariant: arr[lo..i] < pivot, arr[i..j] >= pivot, arr[pivot] = pivot
        let pivot = hi - 1;
        let mut i = lo;
        for j in lo..pivot {
            if arr[j] < arr[pivot] {
                arr.swap(i, j);
                i += 1;
            }
        }
        arr.swap(i, pivot);

        stack.push((lo, i));
        stack.push((i + 1, hi));
    }
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
