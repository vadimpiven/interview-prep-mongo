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
pub struct MergeIterator<I: Iterator<Item = T>, T: Ord> {
    heap: BinaryHeap<Stream<I, T>>,
}

struct Stream<I: Iterator<Item = T>, T: Ord> {
    current: T,
    source_id: usize,
    iter: I,
}

// BinaryHeap is a max-heap. Reverse ordering to get min-heap behavior.
impl<I: Iterator<Item = T>, T: Ord> Ord for Stream<I, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .current
            .cmp(&self.current) // reversed: smaller = higher priority
            .then(other.source_id.cmp(&self.source_id)) // stable: lower id wins ties
    }
}

impl<I: Iterator<Item = T>, T: Ord> PartialOrd for Stream<I, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: Iterator<Item = T>, T: Ord> PartialEq for Stream<I, T> {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current && self.source_id == other.source_id
    }
}

impl<I: Iterator<Item = T>, T: Ord> Eq for Stream<I, T> {}

impl<I: Iterator<Item = T>, T: Ord> MergeIterator<I, T> {
    pub fn new(iters: Vec<I>) -> Self {
        let mut heap = BinaryHeap::new();
        for (id, mut iter) in iters.into_iter().enumerate() {
            if let Some(val) = iter.next() {
                heap.push(Stream {
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

    fn next(&mut self) -> Option<T> {
        let mut stream = self.heap.pop()?;
        let result = if let Some(next_val) = stream.iter.next() {
            // Stream has more — swap current value and re-push
            std::mem::replace(&mut stream.current, next_val)
        } else {
            // Stream exhausted — return current, don't re-push
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
    fn merge_three_streams() {
        let a = vec![1, 4, 7].into_iter();
        let b = vec![2, 5, 8].into_iter();
        let c = vec![3, 6, 9].into_iter();
        let result: Vec<i32> = MergeIterator::new(vec![a, b, c]).collect();
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn merge_with_empty_stream() {
        let a: Vec<i32> = vec![];
        let b = vec![1, 2, 3];
        let result: Vec<i32> = MergeIterator::new(vec![a.into_iter(), b.into_iter()]).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn all_empty() {
        let result: Vec<i32> = MergeIterator::new(Vec::<std::vec::IntoIter<i32>>::new()).collect();
        assert!(result.is_empty());
    }

    #[test]
    fn single_stream() {
        let a = vec![5, 10, 15].into_iter();
        let result: Vec<i32> = MergeIterator::new(vec![a]).collect();
        assert_eq!(result, vec![5, 10, 15]);
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
    fn already_interleaved() {
        let a = vec![1, 2, 3].into_iter();
        let b = vec![1, 2, 3].into_iter();
        let result: Vec<i32> = MergeIterator::new(vec![a, b]).collect();
        assert_eq!(result, vec![1, 1, 2, 2, 3, 3]);
    }
}
