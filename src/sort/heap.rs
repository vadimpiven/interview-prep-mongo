// Binary Heap — generic, comparator-driven, backed by a Vec
//
// This is the shared heap used by:
//   - Heapsort (max-heap: pass normal comparator)
//   - K-way merge (min-heap: pass reversed comparator)
//
// The heap is a "max-heap" with respect to the provided comparator:
// the element for which `cmp` returns Greater wins and rises to the top.
// To get a min-heap, pass a comparator that reverses the ordering.
//
// Operations:
//   - from_vec:         O(n)     — heapify an existing Vec bottom-up
//   - push:             O(log n) — append to end, sift up
//   - pop:              O(log n) — swap root with last, shrink, sift down
//   - into_sorted_vec:  O(n log n) — heapsort: extract all in order

use std::cmp::Ordering;

/// Binary heap backed by a `Vec<T>`, ordered by a caller-supplied comparator.
///
/// With `T::cmp` this is a max-heap (largest at root).
/// With `|a, b| b.cmp(a)` this is a min-heap (smallest at root).
pub struct Heap<T, F: Fn(&T, &T) -> Ordering> {
    data: Vec<T>,
    cmp: F,
}

impl<T, F: Fn(&T, &T) -> Ordering> Heap<T, F> {
    /// Compare two elements by index.
    fn cmp(&self, a: usize, b: usize) -> Ordering {
        (self.cmp)(&self.data[a], &self.data[b])
    }

    /// Build a heap from an existing Vec in O(n).
    pub fn from_vec(data: Vec<T>, cmp: F) -> Self {
        let mut heap = Self { data, cmp };
        let len = heap.data.len();
        if len > 1 {
            for i in (0..=(len - 2) / 2).rev() {
                heap.sift_down(i, len);
            }
        }
        heap
    }

    /// Add an element. O(log n).
    pub fn push(&mut self, val: T) {
        self.data.push(val);
        self.sift_up(self.data.len() - 1);
    }

    /// Remove and return the root element. O(log n).
    pub fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() {
            return None;
        }
        let last = self.data.len() - 1;
        self.data.swap(0, last);
        let val = self.data.pop().unwrap();
        if !self.data.is_empty() {
            self.sift_down(0, self.data.len());
        }
        Some(val)
    }

    /// Sort the heap's data in ascending order (with respect to `cmp`) and return it.
    ///
    /// This is heapsort: repeatedly swap root (max) to the end, shrink heap, sift down.
    /// O(n log n) time.
    pub fn into_sorted_vec(mut self) -> Vec<T> {
        for end in (1..self.data.len()).rev() {
            self.data.swap(0, end);
            self.sift_down(0, end);
        }
        self.data
    }

    /// Sift element at `pos` up toward the root.
    fn sift_up(&mut self, mut pos: usize) {
        while pos > 0 {
            let parent = (pos - 1) / 2;
            if self.cmp(pos, parent).is_le() {
                break;
            }
            self.data.swap(pos, parent);
            pos = parent;
        }
    }

    /// Sift element at `pos` down within `data[..heap_len]`.
    ///
    /// Split into two phases for branch elimination:
    /// 1. While both children exist (no bounds check on right child)
    /// 2. Handle the last level where only a left child might exist
    fn sift_down(&mut self, mut pos: usize, heap_len: usize) {
        // Phase 1: both children guaranteed to exist.
        // `2 * pos + 2 < heap_len` means right child is in bounds,
        // so we can pick the larger of left/right without a bounds check.
        while 2 * pos + 2 < heap_len {
            let left = 2 * pos + 1;
            // Pick larger child: no bounds check needed for right (left + 1).
            let child = if self.cmp(left + 1, left).is_gt() {
                left + 1
            } else {
                left
            };
            if self.cmp(pos, child).is_ge() {
                return;
            }
            self.data.swap(pos, child);
            pos = child;
        }

        // Phase 2: at most a left child exists.
        let left = 2 * pos + 1;
        if left < heap_len && self.cmp(pos, left).is_lt() {
            self.data.swap(pos, left);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_heap_push_pop() {
        let mut h = Heap::from_vec(vec![], i32::cmp);
        h.push(3);
        h.push(1);
        h.push(4);
        h.push(1);
        h.push(5);
        assert_eq!(h.pop(), Some(5));
        assert_eq!(h.pop(), Some(4));
        assert_eq!(h.pop(), Some(3));
        assert_eq!(h.pop(), Some(1));
        assert_eq!(h.pop(), Some(1));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn min_heap_push_pop() {
        let mut h = Heap::from_vec(vec![], |a: &i32, b: &i32| b.cmp(a));
        h.push(3);
        h.push(1);
        h.push(4);
        h.push(1);
        h.push(5);
        assert_eq!(h.pop(), Some(1));
        assert_eq!(h.pop(), Some(1));
        assert_eq!(h.pop(), Some(3));
        assert_eq!(h.pop(), Some(4));
        assert_eq!(h.pop(), Some(5));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn into_sorted_vec() {
        let h = Heap::from_vec(vec![5, 3, 1, 4, 2], i32::cmp);
        assert_eq!(h.into_sorted_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn empty() {
        let mut h = Heap::from_vec(Vec::<i32>::new(), i32::cmp);
        assert_eq!(h.pop(), None);
    }
}
