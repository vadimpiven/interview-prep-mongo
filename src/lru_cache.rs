// LRU Cache — HashMap + VecDeque
//
// MongoDB's plan cache (lru_key_value.h) uses HashMap + doubly-linked list
// for O(1) get/put. This implementation uses VecDeque for simplicity —
// get() is O(n) due to VecDeque::remove(), but correct and easy to write
// in 20 minutes.
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

use std::collections::HashMap;
use std::collections::VecDeque;

pub struct LRUCache<K: Eq + std::hash::Hash + Clone, V> {
    map: HashMap<K, usize>,    // key → index in entries
    entries: VecDeque<(K, V)>, // front = MRU, back = LRU
    max_size: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V> LRUCache<K, V> {
    pub fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::new(),
            entries: VecDeque::new(),
            max_size,
        }
    }

    /// Get value by key. Promotes entry to MRU position.
    /// O(n) due to VecDeque::remove — O(1) with linked list in production.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let idx = *self.map.get(key)?;
        let entry = self.entries.remove(idx)?;
        self.entries.push_front(entry);
        self.rebuild_indices();
        Some(&self.entries.front().unwrap().1)
    }

    /// Insert or overwrite. Evicts LRU if over capacity.
    pub fn put(&mut self, key: K, value: V) {
        if let Some(&idx) = self.map.get(&key) {
            self.entries.remove(idx);
        }
        self.entries.push_front((key.clone(), value));
        self.rebuild_indices();
        self.evict();
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn evict(&mut self) {
        while self.entries.len() > self.max_size {
            if let Some((k, _)) = self.entries.pop_back() {
                self.map.remove(&k);
            }
        }
    }

    /// Rebuild index map after structural changes to VecDeque.
    /// This is the cost of using VecDeque instead of a linked list.
    fn rebuild_indices(&mut self) {
        self.map.clear();
        for (i, (k, _)) in self.entries.iter().enumerate() {
            self.map.insert(k.clone(), i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_get_put() {
        let mut cache = LRUCache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        assert_eq!(cache.get(&1), Some(&10));
        assert_eq!(cache.get(&99), None);
    }

    #[test]
    fn evicts_lru() {
        let mut cache = LRUCache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        cache.put(4, 40); // evicts 1 (least recently used)
        assert!(!cache.contains(&1));
        assert!(cache.contains(&2));
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
    }

    #[test]
    fn get_promotes_to_mru() {
        let mut cache = LRUCache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        cache.get(&1); // promote 1 to MRU
        cache.put(4, 40); // evicts 2 (now LRU), not 1
        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
    }

    #[test]
    fn overwrite_key() {
        let mut cache = LRUCache::new(3);
        cache.put(1, 10);
        cache.put(1, 20);
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
        cache.put(1, 10);
        cache.put(2, 20); // evicts 1
        assert!(!cache.contains(&1));
        assert_eq!(cache.get(&2), Some(&20));
    }

    #[test]
    fn single_put_and_get() {
        let mut cache = LRUCache::new(10);
        cache.put(1, 42);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&1), Some(&42));
    }

    #[test]
    fn overwrite_promotes_to_mru() {
        let mut cache = LRUCache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        cache.put(1, 99); // overwrite 1 → now MRU
        cache.put(4, 40); // evicts 2 (LRU), not 1
        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
        assert_eq!(cache.get(&1), Some(&99));
    }

    #[test]
    fn evict_entire_cache_and_refill() {
        let mut cache = LRUCache::new(2);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30); // evicts 1
        cache.put(4, 40); // evicts 2
        assert!(!cache.contains(&1));
        assert!(!cache.contains(&2));
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn get_nonexistent_does_not_mutate() {
        let mut cache = LRUCache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        assert_eq!(cache.get(&99), None);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn len_tracks_through_operations() {
        let mut cache = LRUCache::new(3);
        assert_eq!(cache.len(), 0);
        cache.put(1, 10);
        assert_eq!(cache.len(), 1);
        cache.put(2, 20);
        assert_eq!(cache.len(), 2);
        cache.put(1, 99); // overwrite, no size change
        assert_eq!(cache.len(), 2);
        cache.put(3, 30);
        assert_eq!(cache.len(), 3);
        cache.put(4, 40); // evicts oldest
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn capacity_one_repeated_access() {
        let mut cache = LRUCache::new(1);
        cache.put(1, 10);
        assert_eq!(cache.get(&1), Some(&10));
        cache.put(1, 20); // overwrite same key
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&1), Some(&20));
    }
}
