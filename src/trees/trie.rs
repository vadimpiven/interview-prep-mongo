// Trie (Prefix Tree) — Insert, search, prefix search, delete
//
// HR prep guide says: "Familiarize yourself with trie-trees."
//
// MongoDB relevance:
//   - Compound index prefix matching: index {a:1, b:1, c:1} serves
//     queries on {a}, {a,b}, {a,b,c} — this is prefix lookup.
//   - Field path matching: "a.b.c" shares prefix "a.b" with "a.b.d"
//   - Query shape grouping: similar queries share common prefixes
//
// Complexity:
//   insert/search/starts_with: O(L) where L = key length
//   space: O(N * L) worst case, but shared prefixes reduce this
//
// Generic over element type E (char, u8, field name, etc.).
// Any type that is Eq + Hash can be a trie edge.

use std::collections::HashMap;
use std::hash::Hash;

struct Node<E> {
    children: HashMap<E, Node<E>>,
    is_end: bool,
    /// Number of words passing through this node (useful for `count_with_prefix`).
    prefix_count: usize,
}

impl<E: Eq + Hash> Node<E> {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_end: false,
            prefix_count: 0,
        }
    }
}

pub struct Trie<E> {
    root: Node<E>,
}

impl<E: Eq + Hash> Trie<E> {
    #[must_use]
    pub fn new() -> Self {
        Self { root: Node::new() }
    }

    /// Insert a key. Returns true if the key was new. O(L).
    pub fn insert(&mut self, key: &[E]) -> bool
    where
        E: Clone,
    {
        if self.find_node(key).is_some_and(|n| n.is_end) {
            return false;
        }
        let mut node = &mut self.root;
        for elem in key {
            node.prefix_count += 1;
            node = node.children.entry(elem.clone()).or_insert_with(Node::new);
        }
        node.prefix_count += 1;
        node.is_end = true;
        true
    }

    /// Returns true if the exact key was inserted. O(L).
    #[must_use]
    pub fn search(&self, key: &[E]) -> bool {
        self.find_node(key).is_some_and(|n| n.is_end)
    }

    /// Returns true if any inserted key starts with this prefix. O(L).
    #[must_use]
    pub fn starts_with(&self, prefix: &[E]) -> bool {
        self.find_node(prefix).is_some()
    }

    /// Count how many inserted keys start with this prefix. O(L).
    #[must_use]
    pub fn count_with_prefix(&self, prefix: &[E]) -> usize {
        self.find_node(prefix).map_or(0, |n| n.prefix_count)
    }

    /// Delete a key. Returns true if it existed. O(L).
    /// Decrements `prefix_count` along the path. Does not reclaim nodes
    /// (acceptable for interview — mention "production would prune empty branches").
    pub fn delete(&mut self, key: &[E]) -> bool {
        if !self.find_node(key).is_some_and(|n| n.is_end) {
            return false;
        }
        let mut node = &mut self.root;
        for elem in key {
            node.prefix_count -= 1;
            node = node.children.get_mut(elem).unwrap();
        }
        node.prefix_count -= 1;
        node.is_end = false;
        true
    }

    /// Navigate to the node at the end of the given prefix, if it exists.
    fn find_node(&self, prefix: &[E]) -> Option<&Node<E>> {
        let mut node = &self.root;
        for elem in prefix {
            node = node.children.get(elem)?;
        }
        Some(node)
    }
}

/// Convenience: `Trie<char>` can accept `&str` directly.
impl Trie<char> {
    pub fn insert_str(&mut self, word: &str) -> bool {
        self.insert(&word.chars().collect::<Vec<_>>())
    }

    pub fn search_str(&self, word: &str) -> bool {
        self.search(&word.chars().collect::<Vec<_>>())
    }

    pub fn starts_with_str(&self, prefix: &str) -> bool {
        self.starts_with(&prefix.chars().collect::<Vec<_>>())
    }

    pub fn count_with_prefix_str(&self, prefix: &str) -> usize {
        self.count_with_prefix(&prefix.chars().collect::<Vec<_>>())
    }

    pub fn delete_str(&mut self, word: &str) -> bool {
        self.delete(&word.chars().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn empty_trie() {
        let trie = Trie::<char>::new();
        assert!(!trie.search_str("anything"));
        assert!(!trie.starts_with_str("a"));
        assert_eq!(trie.count_with_prefix_str(""), 0);
    }

    // ONE
    #[test]
    fn single_word() {
        let mut trie = Trie::new();
        trie.insert_str("hello");
        assert!(trie.search_str("hello"));
        assert!(!trie.search_str("hell")); // prefix exists, word doesn't
        assert!(trie.starts_with_str("hel"));
        assert_eq!(trie.count_with_prefix_str("hello"), 1);
    }

    // MANY + prefix counting
    #[test]
    fn multiple_words_with_shared_prefix() {
        let mut trie = Trie::new();
        trie.insert_str("hello");
        trie.insert_str("help");
        trie.insert_str("heap");
        assert_eq!(trie.count_with_prefix_str("hel"), 2); // hello, help
        assert_eq!(trie.count_with_prefix_str("he"), 3); // hello, help, heap
        assert_eq!(trie.count_with_prefix_str("z"), 0);
    }

    // EDGE: delete + prefix_count consistency
    #[test]
    fn delete_and_count() {
        let mut trie = Trie::new();
        trie.insert_str("hello");
        trie.insert_str("help");
        assert!(trie.delete_str("hello"));
        assert!(!trie.search_str("hello"));
        assert!(trie.search_str("help"));
        assert_eq!(trie.count_with_prefix_str("hel"), 1);
    }

    // EDGE: delete nonexistent
    #[test]
    fn delete_nonexistent() {
        let mut trie = Trie::new();
        trie.insert_str("hello");
        assert!(!trie.delete_str("world"));
        assert!(trie.search_str("hello"));
    }

    // EDGE: duplicate insert is idempotent
    #[test]
    fn duplicate_insert_idempotent() {
        let mut trie = Trie::new();
        trie.insert_str("hello");
        trie.insert_str("hello"); // no-op
        assert_eq!(trie.count_with_prefix_str("hello"), 1);
        assert!(trie.delete_str("hello"));
        assert_eq!(trie.count_with_prefix_str("hello"), 0);
    }

    // Generic: trie over bytes
    #[test]
    fn byte_trie() {
        let mut trie = Trie::new();
        trie.insert(b"hello");
        trie.insert(b"help");
        assert!(trie.search(b"hello"));
        assert!(!trie.search(b"hell"));
        assert_eq!(trie.count_with_prefix(b"hel"), 2);
    }

    // Generic: trie over field names (compound index prefix matching)
    #[test]
    fn field_name_trie() {
        let mut trie = Trie::new();
        // Index {a, b, c}
        trie.insert(&["a", "b", "c"]);
        // Index {a, b}
        trie.insert(&["a", "b"]);
        // Index {x, y}
        trie.insert(&["x", "y"]);

        assert!(trie.search(&["a", "b", "c"]));
        assert!(trie.search(&["a", "b"]));
        assert!(!trie.search(&["a"])); // prefix exists, but not inserted as a key
        assert!(trie.starts_with(&["a"]));
        assert_eq!(trie.count_with_prefix(&["a"]), 2); // {a,b} and {a,b,c}
        assert_eq!(trie.count_with_prefix(&["x"]), 1); // {x,y}
    }
}
