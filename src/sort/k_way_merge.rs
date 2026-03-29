// K-Way Merge — Heap-based merge of K sorted iterators
//
// Used in MongoDB's external sort merge phase (sorter_template_defs.h MergeIterator):
// - Sort chunks in memory, spill sorted runs to disk
// - K-way merge the runs using a min-heap
// Also used for merging change stream events from multiple shards.
//
// Algorithm:
//   1. Push first element from each iterator onto a max-heap (with reversed Ord -> min-heap)
//   2. Pop minimum, output it
//   3. Advance the iterator that produced it; if not exhausted, push next element
//   4. Repeat until heap is empty
//
// Complexity: O(N log K) where N = total elements, K = number of streams
//
// Stability: ties broken by `source_id` (lower = earlier). MongoDB uses this
// to preserve insertion order for equal sort keys ($sort stability guarantee).

use std::cmp::Ordering;

use super::heap::Heap;

/// Merges K sorted iterators into a single sorted iterator.
/// Each `next()` call is O(log K). Full iteration over N total elements is O(N log K).
///
/// Uses the shared `Heap` struct as a min-heap (reversed comparator) — same
/// struct that heapsort uses as a max-heap.
pub struct MergeIterator<I, T> {
    heap: Heap<MergeSource<I, T>, fn(&MergeSource<I, T>, &MergeSource<I, T>) -> Ordering>,
}

struct MergeSource<I, T> {
    current: T,
    source_id: usize,
    iter: I,
}

/// Min-heap comparator for MergeSource: smallest `current` wins,
/// ties broken by lower `source_id` (stability).
///
/// Reversed because `Heap` is a max-heap — the element that compares
/// Greater rises to the top. By reversing, the smallest value wins.
fn merge_source_cmp<I, T: Ord>(a: &MergeSource<I, T>, b: &MergeSource<I, T>) -> Ordering {
    b.current
        .cmp(&a.current)
        .then(b.source_id.cmp(&a.source_id))
}

impl<I: Iterator<Item = T>, T: Ord> MergeIterator<I, T> {
    /// Initialize merge from K iterators. O(K) to build the heap.
    #[must_use]
    pub fn new(iters: Vec<I>) -> Self {
        let sources: Vec<_> = iters
            .into_iter()
            .enumerate()
            .filter_map(|(id, mut iter)| {
                iter.next().map(|val| MergeSource {
                    current: val,
                    source_id: id,
                    iter,
                })
            })
            .collect();
        Self {
            heap: Heap::from_vec(sources, merge_source_cmp),
        }
    }
}

/// Implements `Iterator` so you can use `.collect()`, `.take()`, for loops, etc.
impl<I: Iterator<Item = T>, T: Ord> Iterator for MergeIterator<I, T> {
    type Item = T;

    /// O(log K) per call -- one heap pop + one heap push.
    fn next(&mut self) -> Option<T> {
        let mut source = self.heap.pop()?;
        let result = if let Some(next_val) = source.iter.next() {
            std::mem::replace(&mut source.current, next_val)
        } else {
            return Some(source.current);
        };
        self.heap.push(source);
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn zero_streams() {
        let result: Vec<i32> = MergeIterator::new(Vec::<std::vec::IntoIter<i32>>::new()).collect();
        assert!(result.is_empty());
    }

    // ONE
    #[test]
    fn single_stream() {
        let result: Vec<i32> = MergeIterator::new(vec![vec![1, 2, 3].into_iter()]).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    // MANY
    #[test]
    fn merge_two_sorted() {
        let a = vec![1, 3, 5].into_iter();
        let b = vec![2, 4, 6].into_iter();
        let result: Vec<i32> = MergeIterator::new(vec![a, b]).collect();
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    // EDGE: one stream is empty
    #[test]
    fn merge_with_empty_stream() {
        let a: Vec<i32> = vec![];
        let b = vec![1, 2, 3];
        let result: Vec<i32> = MergeIterator::new(vec![a.into_iter(), b.into_iter()]).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    // EDGE: duplicates across streams -- stability via `source_id`
    #[test]
    fn duplicate_values_stable() {
        let a = vec![1, 1].into_iter();
        let b = vec![1, 1].into_iter();
        let result: Vec<i32> = MergeIterator::new(vec![a, b]).collect();
        assert_eq!(result, vec![1, 1, 1, 1]);
    }
}
