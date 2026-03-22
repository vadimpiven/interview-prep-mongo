// LRU Cache — VecDeque with linear scan
//
// MongoDB's plan cache (lru_key_value.h) uses HashMap + doubly-linked list
// for O(1) get/insert. This implementation uses VecDeque for simplicity —
// get() and insert() are O(n) due to linear scan and VecDeque::remove(),
// but correct and easy to write in 20 minutes.
//
// To discuss: "In production I'd use an intrusive doubly-linked list with
// raw pointers (or a safe wrapper like `lru` crate) for O(1) get. The
// VecDeque approach trades O(n) get for implementation simplicity."
//
// MongoDB plan cache lifecycle:
//   INACTIVE → ACTIVE (validated by replanning)
//   ACTIVE → REPLAN if cached plan 10× worse than expected
//   INVALIDATE on DDL (index create/drop)
//
// Eviction: LRU (least recently used) evicted when size exceeds budget.
// MongoDB uses byte-budget, not count-budget.

use std::collections::VecDeque;

pub struct LRUCache<K, V> {
    entries: VecDeque<(K, V)>, // front = MRU, back = LRU
    capacity: usize,
}

impl<K: Eq, V> LRUCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "LRU cache capacity must be at least 1");
        Self {
            entries: VecDeque::new(),
            capacity,
        }
    }

    /// Get value by key. Promotes entry to MRU position.
    /// O(n) due to linear scan — O(1) with linked list in production.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let idx = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(idx)?;
        self.entries.push_front(entry);
        Some(&self.entries.front().unwrap().1)
    }

    /// Insert or overwrite. Evicts LRU if over capacity.
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(idx) = self.entries.iter().position(|(k, _)| *k == key) {
            self.entries.remove(idx);
        }
        self.entries.push_front((key, value));
        while self.entries.len() > self.capacity {
            self.entries.pop_back();
        }
    }

    #[must_use]
    pub fn contains(&self, key: &K) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_get_insert() {
        let mut cache = LRUCache::new(3);
        cache.insert(1, 10);
        cache.insert(2, 20);
        assert_eq!(cache.get(&1), Some(&10));
        assert_eq!(cache.get(&99), None);
    }

    #[test]
    fn evicts_lru() {
        let mut cache = LRUCache::new(3);
        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30);
        cache.insert(4, 40); // evicts 1 (least recently used)
        assert!(!cache.contains(&1));
        assert!(cache.contains(&2));
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
    }

    #[test]
    fn get_promotes_to_mru() {
        let mut cache = LRUCache::new(3);
        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30);
        cache.get(&1); // promote 1 to MRU
        cache.insert(4, 40); // evicts 2 (now LRU), not 1
        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
    }

    #[test]
    fn overwrite_key() {
        let mut cache = LRUCache::new(3);
        cache.insert(1, 10);
        cache.insert(1, 20);
        assert_eq!(cache.get(&1), Some(&20));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn empty_cache() {
        let mut cache: LRUCache<i32, i32> = LRUCache::new(3);
        assert_eq!(cache.get(&1), None);
        assert!(cache.is_empty());
    }

    #[test]
    fn capacity_one() {
        let mut cache = LRUCache::new(1);
        cache.insert(1, 10);
        cache.insert(2, 20); // evicts 1
        assert!(!cache.contains(&1));
        assert_eq!(cache.get(&2), Some(&20));
    }

    #[test]
    fn single_insert_and_get() {
        let mut cache = LRUCache::new(10);
        cache.insert(1, 42);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&1), Some(&42));
    }

    #[test]
    fn overwrite_promotes_to_mru() {
        let mut cache = LRUCache::new(3);
        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30);
        cache.insert(1, 99); // overwrite 1 → now MRU
        cache.insert(4, 40); // evicts 2 (LRU), not 1
        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
        assert_eq!(cache.get(&1), Some(&99));
    }

    #[test]
    fn evict_entire_cache_and_refill() {
        let mut cache = LRUCache::new(2);
        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30); // evicts 1
        cache.insert(4, 40); // evicts 2
        assert!(!cache.contains(&1));
        assert!(!cache.contains(&2));
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn get_nonexistent_does_not_mutate() {
        let mut cache = LRUCache::new(3);
        cache.insert(1, 10);
        cache.insert(2, 20);
        assert_eq!(cache.get(&99), None);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn len_tracks_through_operations() {
        let mut cache = LRUCache::new(3);
        assert_eq!(cache.len(), 0);
        cache.insert(1, 10);
        assert_eq!(cache.len(), 1);
        cache.insert(2, 20);
        assert_eq!(cache.len(), 2);
        cache.insert(1, 99); // overwrite, no size change
        assert_eq!(cache.len(), 2);
        cache.insert(3, 30);
        assert_eq!(cache.len(), 3);
        cache.insert(4, 40); // evicts oldest
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn capacity_one_repeated_access() {
        let mut cache = LRUCache::new(1);
        cache.insert(1, 10);
        assert_eq!(cache.get(&1), Some(&10));
        cache.insert(1, 20); // overwrite same key
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&1), Some(&20));
    }

    #[test]
    #[should_panic(expected = "capacity must be at least 1")]
    fn capacity_zero_panics() {
        let _cache: LRUCache<i32, i32> = LRUCache::new(0);
    }
}
