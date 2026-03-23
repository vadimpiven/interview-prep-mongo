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
// Scoped for 45 minutes: insert, search, starts_with, delete, count_with_prefix.

use std::collections::HashMap;

/// Each node holds a map of children keyed by character,
/// plus a flag indicating whether this node marks end of an inserted word.
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
    /// Number of words passing through this node (useful for `count_with_prefix`).
    prefix_count: usize,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_end: false,
            prefix_count: 0,
        }
    }
}

pub struct Trie {
    root: TrieNode,
}

impl Trie {
    /// O(1).
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }

    /// Insert a word. Idempotent -- re-inserting the same word is a no-op.
    /// O(L) where L = word length.
    pub fn insert(&mut self, word: &str) {
        if self.search(word) {
            return; // already present -- keep `prefix_count` consistent
        }
        let mut node = &mut self.root;
        for ch in word.chars() {
            node.prefix_count += 1;
            node = node.children.entry(ch).or_insert_with(TrieNode::new);
        }
        node.prefix_count += 1;
        node.is_end = true;
    }

    /// Returns true if the exact word was inserted. O(L).
    #[must_use]
    pub fn search(&self, word: &str) -> bool {
        self.find_node(word).is_some_and(|n| n.is_end)
    }

    /// Returns true if any inserted word starts with this prefix. O(L).
    #[must_use]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.find_node(prefix).is_some()
    }

    /// Count how many inserted words start with this prefix. O(L).
    #[must_use]
    pub fn count_with_prefix(&self, prefix: &str) -> usize {
        self.find_node(prefix).map_or(0, |n| n.prefix_count)
    }

    /// Delete a word. Returns true if it existed. O(L).
    /// Decrements `prefix_count` along the path. Does not reclaim nodes
    /// (acceptable for interview -- mention "production would prune empty branches").
    ///
    /// # Panics
    ///
    /// Panics if the trie's internal structure is inconsistent (child node missing
    /// along a path that was confirmed to exist by `search`).
    pub fn delete(&mut self, word: &str) -> bool {
        if !self.search(word) {
            return false;
        }
        let mut node = &mut self.root;
        for ch in word.chars() {
            node.prefix_count -= 1;
            node = node.children.get_mut(&ch).unwrap();
        }
        node.prefix_count -= 1;
        node.is_end = false;
        true
    }

    /// Collect all words in the trie (for debugging/testing). O(total chars).
    #[must_use]
    pub fn collect_all(&self) -> Vec<String> {
        let mut results = Vec::new();
        collect_recursive(&self.root, &mut String::new(), &mut results);
        results
    }

    /// Navigate to the node at the end of the given prefix, if it exists.
    fn find_node(&self, prefix: &str) -> Option<&TrieNode> {
        let mut node = &self.root;
        for ch in prefix.chars() {
            node = node.children.get(&ch)?;
        }
        Some(node)
    }
}

fn collect_recursive(node: &TrieNode, current: &mut String, results: &mut Vec<String>) {
    if node.is_end {
        results.push(current.clone());
    }
    // Sort keys for deterministic output order
    let mut keys: Vec<char> = node.children.keys().copied().collect();
    keys.sort_unstable();
    for ch in keys {
        current.push(ch);
        collect_recursive(&node.children[&ch], current, results);
        current.pop();
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn empty_trie() {
        let trie = Trie::new();
        assert!(!trie.search("anything"));
        assert!(!trie.starts_with("a"));
        assert_eq!(trie.count_with_prefix(""), 0);
    }

    // ONE
    #[test]
    fn single_word() {
        let mut trie = Trie::new();
        trie.insert("hello");
        assert!(trie.search("hello"));
        assert!(!trie.search("hell")); // prefix exists, word doesn't
        assert!(trie.starts_with("hel"));
        assert_eq!(trie.count_with_prefix("hello"), 1);
    }

    // MANY + prefix counting
    #[test]
    fn multiple_words_with_shared_prefix() {
        let mut trie = Trie::new();
        trie.insert("hello");
        trie.insert("help");
        trie.insert("heap");
        assert_eq!(trie.count_with_prefix("hel"), 2); // hello, help
        assert_eq!(trie.count_with_prefix("he"), 3); // hello, help, heap
        assert_eq!(trie.count_with_prefix("z"), 0);
    }

    // EDGE: delete + prefix_count consistency
    #[test]
    fn delete_and_count() {
        let mut trie = Trie::new();
        trie.insert("hello");
        trie.insert("help");
        assert!(trie.delete("hello"));
        assert!(!trie.search("hello"));
        assert!(trie.search("help"));
        assert_eq!(trie.count_with_prefix("hel"), 1);
    }

    // EDGE: delete nonexistent
    #[test]
    fn delete_nonexistent() {
        let mut trie = Trie::new();
        trie.insert("hello");
        assert!(!trie.delete("world"));
        assert!(trie.search("hello"));
    }

    // EDGE: duplicate insert is idempotent
    #[test]
    fn duplicate_insert_idempotent() {
        let mut trie = Trie::new();
        trie.insert("hello");
        trie.insert("hello"); // no-op
        assert_eq!(trie.count_with_prefix("hello"), 1);
        assert!(trie.delete("hello"));
        assert_eq!(trie.count_with_prefix("hello"), 0);
    }
}
