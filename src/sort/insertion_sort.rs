// Insertion Sort — Stable, O(n^2) worst case, O(1) extra space
//
// Simple quadratic sort that is fast for small arrays (< ~16 elements)
// due to low overhead and excellent cache locality. This is why introsort
// switches to insertion sort for small partitions — the constant factor
// beats O(n log n) algorithms at tiny sizes.
//
// Properties:
//   - Stable: equal elements keep their original order
//   - Adaptive: O(n) on already-sorted input (only compares, no moves)
//   - In-place: O(1) extra space
//   - Online: can sort a stream element-by-element
//
// Algorithm:
//   Walk left to right. For each element, shift it left until it reaches
//   its correct position among the already-sorted prefix.

/// Insertion sort -- stable, O(n^2) worst case, O(n) best case (sorted input).
pub fn insertion_sort<T: Ord>(arr: &mut [T]) {
    insertion_sort_by(arr, T::cmp);
}

/// Insertion sort with custom comparator.
pub fn insertion_sort_by<T, F: Fn(&T, &T) -> std::cmp::Ordering>(arr: &mut [T], cmp: F) {
    for i in 1..arr.len() {
        // Shift arr[i] left until it's in the right place.
        let mut j = i;
        while j > 0 && cmp(&arr[j - 1], &arr[j]).is_gt() {
            arr.swap(j - 1, j);
            j -= 1;
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
        insertion_sort(&mut arr);
        assert!(arr.is_empty());
    }

    // ONE
    #[test]
    fn single() {
        let mut arr = vec![42];
        insertion_sort(&mut arr);
        assert_eq!(arr, vec![42]);
    }

    // MANY
    #[test]
    fn basic() {
        let mut arr = vec![5, 3, 1, 4, 2];
        insertion_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: already sorted (best case — O(n))
    #[test]
    fn already_sorted() {
        let mut arr = vec![1, 2, 3, 4, 5];
        insertion_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: reverse sorted (worst case — O(n^2))
    #[test]
    fn reverse() {
        let mut arr = vec![5, 4, 3, 2, 1];
        insertion_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    // EDGE: duplicates
    #[test]
    fn duplicates() {
        let mut arr = vec![3, 1, 2, 1, 3, 2];
        insertion_sort(&mut arr);
        assert_eq!(arr, vec![1, 1, 2, 2, 3, 3]);
    }

    // EDGE: all equal
    #[test]
    fn all_same() {
        let mut arr = vec![7, 7, 7, 7];
        insertion_sort(&mut arr);
        assert_eq!(arr, vec![7, 7, 7, 7]);
    }

    // EDGE: stability — equal elements preserve original order
    #[test]
    fn stable() {
        let mut arr = vec![(3, 0), (1, 1), (3, 2), (1, 3)];
        insertion_sort_by(&mut arr, |a, b| a.0.cmp(&b.0));
        assert_eq!(arr, vec![(1, 1), (1, 3), (3, 0), (3, 2)]);
    }
}
