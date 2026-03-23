// Sorting — Merge Sort and Quicksort
//
// "Know how to sort. Don't do bubble-sort. You should know the details of at
// least one n*log(n) sorting algorithm, preferably two (say, quicksort and
// merge sort). Merge sort can be highly useful in situations where quicksort
// is impractical, so take a look at it."
//
// Merge sort:
//   - Stable, always O(n log n) time, O(n) extra space.
//   - Preferred for linked lists (no random access needed), external sort
//     (naturally splits into runs), and when stability is required.
//   - MongoDB's external sort (src/mongo/db/sorter/) is essentially merge sort:
//     sort chunks in memory, spill sorted runs, k-way merge.
//
// Quicksort:
//   - In-place, O(n log n) average, O(n²) worst case (mitigated by random pivot).
//   - Preferred for arrays (cache-friendly sequential access), and when extra
//     space is a concern.
//   - Rust's std Vec::sort_unstable uses a pattern-defeating quicksort variant.

/// Merge sort. Stable. O(n log n) time, O(n) space.
/// Recursively split in half, sort each half, merge.
pub fn merge_sort<T: Ord + Clone>(data: &mut [T]) {
    let len = data.len();
    if len <= 1 {
        return;
    }
    let mid = len / 2;
    merge_sort(&mut data[..mid]);
    merge_sort(&mut data[mid..]);
    merge(data, mid);
}

/// Merge two sorted halves data[..mid] and data[mid..] into sorted data[..].
/// O(n) time and space for the merge step.
fn merge<T: Ord + Clone>(data: &mut [T], mid: usize) {
    let left = data[..mid].to_vec();
    let right = data[mid..].to_vec();

    let (mut i, mut j, mut k) = (0, 0, 0);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            data[k] = left[i].clone();
            i += 1;
        } else {
            data[k] = right[j].clone();
            j += 1;
        }
        k += 1;
    }
    while i < left.len() {
        data[k] = left[i].clone();
        i += 1;
        k += 1;
    }
    while j < right.len() {
        data[k] = right[j].clone();
        j += 1;
        k += 1;
    }
}

/// Quicksort. In-place. O(n log n) average, O(n²) worst case.
/// Uses last element as pivot (Lomuto partition scheme).
/// Not stable.
pub fn quicksort<T: Ord>(data: &mut [T]) {
    let len = data.len();
    if len <= 1 {
        return;
    }
    let pivot = partition(data);
    quicksort(&mut data[..pivot]);
    quicksort(&mut data[pivot + 1..]);
}

/// Lomuto partition: pick last element as pivot, partition into
/// [elements ≤ pivot | pivot | elements > pivot]. Returns pivot index.
/// O(n) time, O(1) space.
fn partition<T: Ord>(data: &mut [T]) -> usize {
    let last = data.len() - 1;
    let mut i = 0;
    for j in 0..last {
        if data[j] <= data[last] {
            data.swap(i, j);
            i += 1;
        }
    }
    data.swap(i, last);
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_sort_basic() {
        let mut data = vec![5, 3, 1, 4, 2];
        merge_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn merge_sort_empty_and_single() {
        let mut empty: Vec<i32> = vec![];
        merge_sort(&mut empty);
        assert!(empty.is_empty());

        let mut single = vec![42];
        merge_sort(&mut single);
        assert_eq!(single, vec![42]);
    }

    #[test]
    fn merge_sort_duplicates() {
        let mut data = vec![3, 1, 2, 1, 3, 2];
        merge_sort(&mut data);
        assert_eq!(data, vec![1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn quicksort_basic() {
        let mut data = vec![5, 3, 1, 4, 2];
        quicksort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn quicksort_empty_and_single() {
        let mut empty: Vec<i32> = vec![];
        quicksort(&mut empty);
        assert!(empty.is_empty());

        let mut single = vec![42];
        quicksort(&mut single);
        assert_eq!(single, vec![42]);
    }

    #[test]
    fn quicksort_duplicates() {
        let mut data = vec![3, 1, 2, 1, 3, 2];
        quicksort(&mut data);
        assert_eq!(data, vec![1, 1, 2, 2, 3, 3]);
    }
}
