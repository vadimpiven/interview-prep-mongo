// Hash Map — Open Addressing with Linear Probing
//
// Matches the design of absl::flat_hash_map used throughout MongoDB:
// - Flat array of slots (cache-friendly, no pointer chasing)
// - Linear probing for collision resolution
// - Power-of-2 capacity for fast modulo via bitmask
// - Tombstone markers for deletion (lazy deletion)
// - Rehash at 75% load factor
//
// Complexity:
//   insert/get/remove: O(1) amortized, O(n) worst case
//   rehash: O(n)
//
// Alternative designs to discuss:
//   - Chaining (std::unordered_map): simpler deletion, worse cache locality
//   - Robin Hood hashing: reduces probe length variance, more complex insert
//   - Swiss table (what absl actually uses): SIMD metadata, group probing

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct HashMap<K, V> {
    slots: Vec<Option<Slot<K, V>>>,
    size: usize,
    capacity: usize,
}

struct Slot<K, V> {
    key: K,
    value: V,
    deleted: bool,
}

impl<K: Hash + Eq + Clone, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        let mut slots = Vec::with_capacity(capacity);
        slots.resize_with(capacity, || None);
        Self {
            slots,
            size: 0,
            capacity,
        }
    }

    /// Insert or overwrite. Rehashes if load factor exceeds 75%.
    pub fn insert(&mut self, key: K, value: V) {
        if self.size * 4 >= self.capacity * 3 {
            self.rehash(self.capacity * 2);
        }
        let idx = self.probe(&key);
        match &mut self.slots[idx] {
            Some(slot) if !slot.deleted && slot.key == key => {
                slot.value = value; // overwrite existing
            }
            _ => {
                self.slots[idx] = Some(Slot {
                    key,
                    value,
                    deleted: false,
                });
                self.size += 1;
            }
        }
    }

    /// Returns reference to value, or None if not found.
    pub fn get(&self, key: &K) -> Option<&V> {
        let idx = self.probe(key);
        match &self.slots[idx] {
            Some(slot) if !slot.deleted && slot.key == *key => Some(&slot.value),
            _ => None,
        }
    }

    /// Tombstone deletion. Returns true if key existed.
    pub fn remove(&mut self, key: &K) -> bool {
        let idx = self.probe(key);
        match &mut self.slots[idx] {
            Some(slot) if !slot.deleted && slot.key == *key => {
                slot.deleted = true;
                self.size -= 1;
                true
            }
            _ => false,
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Linear probe: walk forward until we find the key, an empty slot,
    /// or a tombstone (reusable slot).
    fn probe(&self, key: &K) -> usize {
        let mut idx = self.hash_key(key) & (self.capacity - 1);
        loop {
            match &self.slots[idx] {
                None => return idx,
                Some(slot) if slot.deleted => return idx,
                Some(slot) if slot.key == *key => return idx,
                _ => idx = (idx + 1) & (self.capacity - 1),
            }
        }
    }

    fn hash_key(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// Allocate new backing array, re-insert all live slots.
    fn rehash(&mut self, new_capacity: usize) {
        let old_slots = std::mem::replace(&mut self.slots, {
            let mut v = Vec::with_capacity(new_capacity);
            v.resize_with(new_capacity, || None);
            v
        });
        self.capacity = new_capacity;
        self.size = 0;
        for slot in old_slots.into_iter().flatten() {
            if !slot.deleted {
                self.insert(slot.key, slot.value);
            }
        }
    }
}

impl<K: Hash + Eq + Clone, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut map = HashMap::new();
        map.insert(1, "one");
        map.insert(2, "two");
        assert_eq!(map.get(&1), Some(&"one"));
        assert_eq!(map.get(&2), Some(&"two"));
        assert_eq!(map.get(&99), None);
    }

    #[test]
    fn overwrite_key() {
        let mut map = HashMap::new();
        map.insert(1, 10);
        map.insert(1, 20);
        assert_eq!(map.get(&1), Some(&20));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn remove_and_reinsert() {
        let mut map = HashMap::new();
        map.insert(1, 10);
        assert!(map.remove(&1));
        assert_eq!(map.get(&1), None);
        assert_eq!(map.len(), 0);
        map.insert(1, 20);
        assert_eq!(map.get(&1), Some(&20));
    }

    #[test]
    fn remove_nonexistent() {
        let mut map: HashMap<i32, i32> = HashMap::new();
        assert!(!map.remove(&1));
    }

    #[test]
    fn rehash_on_load() {
        let mut map = HashMap::with_capacity(4);
        for i in 0..100 {
            map.insert(i, i * 10);
        }
        for i in 0..100 {
            assert_eq!(map.get(&i), Some(&(i * 10)), "Missing key {}", i);
        }
        assert_eq!(map.len(), 100);
    }

    #[test]
    fn empty_map() {
        let map: HashMap<i32, i32> = HashMap::new();
        assert_eq!(map.get(&1), None);
        assert!(map.is_empty());
    }

    #[test]
    fn string_keys() {
        let mut map = HashMap::new();
        map.insert("hello".to_string(), 1);
        map.insert("world".to_string(), 2);
        assert_eq!(map.get(&"hello".to_string()), Some(&1));
        assert_eq!(map.get(&"missing".to_string()), None);
    }
}
