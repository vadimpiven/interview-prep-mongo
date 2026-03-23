// K-Way Merge — Heap-based merge of K sorted iterators
//
// Used in MongoDB's external sort merge phase (sorter_template_defs.h MergeIterator):
// - Sort chunks in memory, spill sorted runs to disk
// - K-way merge the runs using a min-heap
// Also used for merging change stream events from multiple shards.
//
// Algorithm:
//   1. Push first element from each iterator onto a max-heap (with reversed Ord → min-heap)
//   2. Pop minimum, output it
//   3. Advance the iterator that produced it; if not exhausted, push next element
//   4. Repeat until heap is empty
//
// Complexity: O(N log K) where N = total elements, K = number of streams
//
// Stability: ties broken by source_id (lower = earlier). MongoDB uses this
// to preserve insertion order for equal sort keys ($sort stability guarantee).

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Merges K sorted iterators into a single sorted iterator.
/// Each next() call is O(log K). Full iteration over N total elements is O(N log K).
pub struct MergeIterator<I, T> {
    heap: BinaryHeap<MergeSource<I, T>>,
}

struct MergeSource<I, T> {
    current: T,
    source_id: usize,
    iter: I,
}

// BinaryHeap is a max-heap. Reverse ordering to get min-heap behavior.
impl<I: Iterator<Item = T>, T: Ord> Ord for MergeSource<I, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .current
            .cmp(&self.current) // reversed: smaller = higher priority
            .then(other.source_id.cmp(&self.source_id)) // stable: lower id wins ties
    }
}

impl<I: Iterator<Item = T>, T: Ord> PartialOrd for MergeSource<I, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: Iterator<Item = T>, T: Ord> PartialEq for MergeSource<I, T> {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current && self.source_id == other.source_id
    }
}

impl<I: Iterator<Item = T>, T: Ord> Eq for MergeSource<I, T> {}

impl<I: Iterator<Item = T>, T: Ord> MergeIterator<I, T> {
    /// Initialize merge from K iterators. O(K log K) to build the heap.
    pub fn new(iters: Vec<I>) -> Self {
        let mut heap = BinaryHeap::new();
        for (id, mut iter) in iters.into_iter().enumerate() {
            if let Some(val) = iter.next() {
                heap.push(MergeSource {
                    current: val,
                    source_id: id,
                    iter,
                });
            }
        }
        Self { heap }
    }
}

/// Implements Iterator so you can use .collect(), .take(), for loops, etc.
impl<I: Iterator<Item = T>, T: Ord> Iterator for MergeIterator<I, T> {
    type Item = T;

    /// O(log K) per call — one heap pop + one heap push.
    fn next(&mut self) -> Option<T> {
        let mut stream = self.heap.pop()?;
        let result = if let Some(next_val) = stream.iter.next() {
            // MergeSource has more — swap current value and re-push
            std::mem::replace(&mut stream.current, next_val)
        } else {
            // MergeSource exhausted — return current, don't re-push
            return Some(stream.current);
        };
        self.heap.push(stream);
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_two_sorted() {
        let a = vec![1, 3, 5].into_iter();
        let b = vec![2, 4, 6].into_iter();
        let result: Vec<i32> = MergeIterator::new(vec![a, b]).collect();
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn merge_with_empty_stream() {
        let a: Vec<i32> = vec![];
        let b = vec![1, 2, 3];
        let result: Vec<i32> = MergeIterator::new(vec![a.into_iter(), b.into_iter()]).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn duplicate_values_stable() {
        // source_id 0 should come before source_id 1 for equal values
        let a = vec![1, 1].into_iter(); // source 0
        let b = vec![1, 1].into_iter(); // source 1
        let result: Vec<i32> = MergeIterator::new(vec![a, b]).collect();
        assert_eq!(result, vec![1, 1, 1, 1]);
    }

    #[test]
    fn unbalanced_stream_lengths() {
        let a = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10].into_iter();
        let b = vec![5].into_iter();
        let c = vec![3, 11].into_iter();
        let result: Vec<i32> = MergeIterator::new(vec![a, b, c]).collect();
        assert_eq!(result, vec![1, 2, 3, 3, 4, 5, 5, 6, 7, 8, 9, 10, 11]);
    }
}
