// Hash Map — Open Addressing with Linear Probing
//
// Matches the design of absl::flat_hash_map used throughout MongoDB:
// - Flat array of slots (cache-friendly, no pointer chasing)
// - Linear probing for collision resolution
// - Power-of-2 capacity for fast modulo via bitmask
// - Tombstone markers for deletion (lazy deletion)
// - Rehash at 75% load factor (counting both live entries and tombstones)
//
// Complexity:
//   insert/get/remove: O(1) amortized, O(n) worst case
//   rehash: O(n)
//
// Alternative designs to discuss:
//   - Chaining (std::unordered_map): simpler deletion, worse cache locality
//   - Robin Hood hashing: reduces probe length variance, more complex insert
//   - Swiss table (what absl actually uses): SIMD metadata, group probing

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};

pub struct HashMap<K, V, S = RandomState> {
    slots: Vec<Option<Slot<K, V>>>,
    len: usize,
    tombstone_count: usize,
    hash_builder: S,
}

#[derive(Debug)]
struct Slot<K, V> {
    key: K,
    value: V,
    deleted: bool,
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    /// Zero-cost construction — no allocation until first insert.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// O(c) where c = capacity rounded up to next power of two.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> HashMap<K, V, S> {
    /// O(c) where c = capacity rounded up to next power of two (0 = no allocation).
    #[must_use]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        let capacity = if capacity == 0 {
            0
        } else {
            capacity.next_power_of_two()
        };
        Self {
            slots: (0..capacity).map(|_| None).collect(),
            len: 0,
            tombstone_count: 0,
            hash_builder,
        }
    }

    /// Insert or overwrite. Rehashes if load factor exceeds 75%.
    /// O(1) amortized, O(n) worst case (due to probing or rehash).
    pub fn insert(&mut self, key: K, value: V) {
        if self.capacity() == 0 || (self.len + self.tombstone_count) * 4 >= self.capacity() * 3 {
            self.rehash((self.capacity() * 2).max(16));
        }
        let idx = self.probe_insert(&key);
        match &mut self.slots[idx] {
            Some(slot) if !slot.deleted && slot.key == key => {
                slot.value = value; // overwrite existing
            }
            Some(slot) if slot.deleted => {
                // Reuse tombstone slot
                slot.key = key;
                slot.value = value;
                slot.deleted = false;
                self.len += 1;
                self.tombstone_count -= 1;
            }
            _ => {
                self.slots[idx] = Some(Slot {
                    key,
                    value,
                    deleted: false,
                });
                self.len += 1;
            }
        }
    }

    /// Returns reference to value, or `None` if not found.
    /// O(1) amortized, O(n) worst case (due to probing).
    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        if self.slots.is_empty() {
            return None;
        }
        let idx = self.probe_lookup(key);
        match &self.slots[idx] {
            Some(slot) if !slot.deleted && slot.key == *key => Some(&slot.value),
            _ => None,
        }
    }

    /// Tombstone deletion. Returns true if key existed.
    /// O(1) amortized, O(n) worst case (due to probing).
    pub fn remove(&mut self, key: &K) -> bool {
        if self.slots.is_empty() {
            return false;
        }
        let idx = self.probe_lookup(key);
        match &mut self.slots[idx] {
            Some(slot) if !slot.deleted && slot.key == *key => {
                slot.deleted = true;
                self.len -= 1;
                self.tombstone_count += 1;
                true
            }
            _ => false,
        }
    }

    /// O(1).
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// O(1).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return an iterator over live (key, value) pairs.
    /// O(n) to fully iterate (scans all slots including empty/tombstone).
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots.iter().filter_map(|slot| match slot {
            Some(s) if !s.deleted => Some((&s.key, &s.value)),
            _ => None,
        })
    }

    fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Probe for lookup: skip tombstones, stop at empty slot or key match.
    /// Must not stop at tombstones — the target key may be further along
    /// the probe chain (inserted before the tombstoned slot was deleted).
    fn probe_lookup(&self, key: &K) -> usize {
        let mask = self.capacity() - 1;
        let mut idx = self.compute_hash(key) & mask;
        loop {
            match &self.slots[idx] {
                None => return idx,
                Some(slot) if !slot.deleted && slot.key == *key => return idx,
                _ => idx = (idx + 1) & mask,
            }
        }
    }

    /// Probe for insert: remember first tombstone, but keep probing for
    /// an existing live key. This prevents creating duplicate entries when
    /// a tombstone precedes the live entry in the probe chain.
    fn probe_insert(&self, key: &K) -> usize {
        let mask = self.capacity() - 1;
        let mut idx = self.compute_hash(key) & mask;
        let mut first_tombstone: Option<usize> = None;
        loop {
            match &self.slots[idx] {
                None => return first_tombstone.unwrap_or(idx),
                Some(slot) if slot.deleted => {
                    if first_tombstone.is_none() {
                        first_tombstone = Some(idx);
                    }
                    idx = (idx + 1) & mask;
                }
                Some(slot) if slot.key == *key => return idx,
                _ => idx = (idx + 1) & mask,
            }
        }
    }

    fn compute_hash(&self, key: &K) -> usize {
        // Intentional truncation on 32-bit targets: hash values are used modulo capacity.
        #[allow(clippy::cast_possible_truncation)]
        let hash = self.hash_builder.hash_one(key) as usize;
        hash
    }

    /// Allocate new backing array, re-insert all live slots.
    /// Clears all tombstones.
    fn rehash(&mut self, new_capacity: usize) {
        let old_slots =
            std::mem::replace(&mut self.slots, (0..new_capacity).map(|_| None).collect());
        self.len = 0;
        self.tombstone_count = 0;
        for slot in old_slots.into_iter().flatten() {
            if !slot.deleted {
                self.insert(slot.key, slot.value);
            }
        }
    }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> Default for HashMap<K, V, S> {
    fn default() -> Self {
        Self::with_capacity_and_hasher(16, S::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::Hasher;

    /// Deterministic hasher that maps all keys to the same bucket,
    /// forcing maximum collisions for testing probe chain behavior.
    #[derive(Clone)]
    struct CollisionHasher;
    impl BuildHasher for CollisionHasher {
        type Hasher = FixedHasher;
        fn build_hasher(&self) -> FixedHasher {
            FixedHasher
        }
    }
    struct FixedHasher;
    impl Hasher for FixedHasher {
        fn finish(&self) -> u64 {
            0 // all keys hash to bucket 0
        }
        fn write(&mut self, _bytes: &[u8]) {}
    }

    // ZERO
    #[test]
    fn empty_map() {
        let map: HashMap<i32, i32> = HashMap::new();
        assert_eq!(map.get(&1), None);
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    // MANY
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
    fn get_survives_tombstone_in_probe_chain() {
        let mut map = HashMap::with_capacity(16);
        for i in 0..10 {
            map.insert(i, i * 10);
        }
        map.remove(&3);
        map.remove(&5);
        map.remove(&7);
        for i in 0..10 {
            if i == 3 || i == 5 || i == 7 {
                assert_eq!(map.get(&i), None);
            } else {
                assert_eq!(map.get(&i), Some(&(i * 10)), "Missing key {}", i);
            }
        }
    }

    #[test]
    fn probe_insert_no_duplicate_through_tombstone() {
        // Force all keys to collide. Insert A, B (both at bucket 0).
        // Remove A (tombstone at 0, B live at 1). Overwrite B.
        // Must NOT create a duplicate — len stays 1.
        let mut map = HashMap::with_capacity_and_hasher(8, CollisionHasher);
        map.insert(1, 10); // slot 0
        map.insert(2, 20); // slot 1
        assert_eq!(map.len(), 2);

        map.remove(&1); // tombstone at slot 0
        assert_eq!(map.len(), 1);

        map.insert(2, 99); // must find live 2 at slot 1, NOT reuse tombstone at 0
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&2), Some(&99));

        // After removing 2, it must be fully gone (no ghost duplicate)
        assert!(map.remove(&2));
        assert_eq!(map.get(&2), None);
        assert_eq!(map.len(), 0);
    }
}
