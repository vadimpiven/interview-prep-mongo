// LRU Cache — HashMap + Index-Based Doubly-Linked List
//
// MongoDB's plan cache (lru_key_value.h) uses HashMap + doubly-linked list
// for O(1) get/insert. This implementation achieves the same O(1) complexity
// using a Vec as an arena for linked list nodes, with indices instead of
// raw pointers — fully safe Rust, no unsafe blocks.
//
// Data structure:
//   HashMap<K, usize>  — maps keys to node indices in the arena (O(1) lookup)
//   Vec<Node<K, V>>    — arena-allocated doubly-linked list nodes
//   head/tail sentinels — avoid edge cases in link/unlink operations
//
// MongoDB plan cache lifecycle:
//   INACTIVE → ACTIVE (validated by replanning)
//   ACTIVE → REPLAN if cached plan 10× worse than expected
//   INVALIDATE on DDL (index create/drop)
//
// Eviction: LRU (least recently used) evicted when size exceeds budget.
// MongoDB uses byte-budget, not count-budget.

use std::collections::HashMap;
use std::hash::Hash;

/// Sentinel value indicating no link (like null pointer).
const NONE: usize = usize::MAX;

pub struct LRUCache<K, V> {
    map: HashMap<K, usize>, // key → node index in arena
    nodes: Vec<Node<K, V>>, // arena for linked list nodes
    head: usize,            // index of MRU node (front of list)
    tail: usize,            // index of LRU node (back of list)
    capacity: usize,
    free: Vec<usize>, // recycled node indices
}

struct Node<K, V> {
    key: K,
    value: V,
    prev: usize, // NONE if this is the head
    next: usize, // NONE if this is the tail
}

impl<K: Eq + Hash + Clone, V> LRUCache<K, V> {
    /// O(1).
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "LRU cache capacity must be at least 1");
        Self {
            map: HashMap::with_capacity(capacity),
            nodes: Vec::with_capacity(capacity),
            head: NONE,
            tail: NONE,
            capacity,
            free: Vec::new(),
        }
    }

    /// Get value by key. Promotes entry to MRU position (head of list).
    /// O(1) — HashMap lookup + linked list unlink/relink.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let &idx = self.map.get(key)?;
        self.move_to_head(idx);
        Some(&self.nodes[idx].value)
    }

    /// Insert or overwrite. Evicts LRU (tail) if over capacity.
    /// O(1) — HashMap insert + linked list operations.
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(&idx) = self.map.get(&key) {
            // Overwrite existing: update value, promote to MRU
            self.nodes[idx].value = value;
            self.move_to_head(idx);
            return;
        }

        // Evict LRU if at capacity
        if self.map.len() >= self.capacity {
            let lru_idx = self.tail;
            debug_assert!(lru_idx != NONE);
            self.unlink(lru_idx);
            let evicted_key = self.nodes[lru_idx].key.clone();
            self.map.remove(&evicted_key);
            self.free.push(lru_idx);
        }

        // Allocate node (reuse freed slot or push new)
        let idx = if let Some(free_idx) = self.free.pop() {
            self.nodes[free_idx] = Node {
                key: key.clone(),
                value,
                prev: NONE,
                next: NONE,
            };
            free_idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Node {
                key: key.clone(),
                value,
                prev: NONE,
                next: NONE,
            });
            idx
        };

        self.push_head(idx);
        self.map.insert(key, idx);
    }

    /// O(1) — HashMap lookup.
    #[must_use]
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// O(1).
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// O(1).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    // -- Internal linked list operations --

    /// Remove node from its current position in the list.
    fn unlink(&mut self, idx: usize) {
        let prev = self.nodes[idx].prev;
        let next = self.nodes[idx].next;

        if prev != NONE {
            self.nodes[prev].next = next;
        } else {
            self.head = next; // was the head
        }

        if next != NONE {
            self.nodes[next].prev = prev;
        } else {
            self.tail = prev; // was the tail
        }

        self.nodes[idx].prev = NONE;
        self.nodes[idx].next = NONE;
    }

    /// Insert node at the head of the list (MRU position).
    fn push_head(&mut self, idx: usize) {
        self.nodes[idx].prev = NONE;
        self.nodes[idx].next = self.head;

        if self.head != NONE {
            self.nodes[self.head].prev = idx;
        }
        self.head = idx;

        if self.tail == NONE {
            self.tail = idx; // first node in the list
        }
    }

    /// Move an existing node to the head (MRU promotion).
    fn move_to_head(&mut self, idx: usize) {
        if self.head == idx {
            return; // already at head
        }
        self.unlink(idx);
        self.push_head(idx);
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

    #[test]
    fn heavy_churn_reuses_freed_slots() {
        let mut cache = LRUCache::new(2);
        for i in 0..100 {
            cache.insert(i, i * 10);
        }
        // Only last 2 should survive
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&99), Some(&990));
        assert_eq!(cache.get(&98), Some(&980));
        assert_eq!(cache.get(&97), None);
    }

    #[test]
    fn interleaved_get_and_insert() {
        let mut cache = LRUCache::new(3);
        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30);
        // Access pattern: touch 1, insert 4 (evicts 2), touch 3, insert 5 (evicts 1)
        cache.get(&1);
        cache.insert(4, 40);
        assert!(!cache.contains(&2)); // 2 was LRU
        cache.get(&3);
        cache.insert(5, 50);
        assert!(!cache.contains(&1)); // 1 was LRU after 3 and 4 were touched
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
        assert!(cache.contains(&5));
    }
}
