// AVL Tree — Self-balancing BST, O(log n) guaranteed
//
// Invariant: for every node, |height(left) - height(right)| <= 1.
// After each insert/delete, walk back up and rotate to restore balance.
//
// Four rotation cases:
//   LL (right rotate)  — left child is too tall, left-left grandchild is tallest
//   RR (left rotate)   — right child is too tall, right-right grandchild is tallest
//   LR (left-right)    — left child is too tall, left-right grandchild is tallest
//   RL (right-left)    — right child is too tall, right-left grandchild is tallest
//
// Height of a balanced AVL tree: at most ~1.44 * log2(n) — tighter than red-black.
// Faster lookups than red-black, but more rotations on insert/delete.
//
// MongoDB relevance:
//   MongoDB uses B-trees (WiredTiger) for indexes, not AVL trees.
//   AVL is relevant as interview prep: "know at least one balanced BST."

use std::cmp::Ordering;

type Link<K, V> = Option<Box<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    height: usize,
    left: Link<K, V>,
    right: Link<K, V>,
}

impl<K: Ord, V> Node<K, V> {
    fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            height: 1,
            left: None,
            right: None,
        }
    }

    fn link_height(link: &Link<K, V>) -> usize {
        link.as_ref().map_or(0, |n| n.height)
    }

    fn update_height(&mut self) {
        self.height = 1 + Self::link_height(&self.left).max(Self::link_height(&self.right));
    }

    /// Returns the balance state: Greater if left-heavy, Less if right-heavy,
    /// Equal if balanced (height difference <= 1).
    fn balance(&self) -> Ordering {
        let left_height = Self::link_height(&self.left);
        let right_height = Self::link_height(&self.right);
        if left_height > right_height + 1 {
            Ordering::Greater
        } else if right_height > left_height + 1 {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }

    // Right rotation (fixes LL imbalance):
    //
    //       self          left
    //      /    \        /    \
    //    left    C  =>  A    self
    //   /    \              /    \
    //  A      B            B      C
    fn rotate_right(mut self: Box<Self>) -> Box<Self> {
        let Some(mut left) = self.left.take() else {
            return self;
        };
        self.left = left.right.take();
        self.update_height();
        left.right = Some(self);
        left.update_height();
        left
    }

    // Left rotation (fixes RR imbalance):
    //
    //    self            right
    //   /    \          /     \
    //  A    right  =>  self    C
    //       /   \     /    \
    //      B     C   A      B
    fn rotate_left(mut self: Box<Self>) -> Box<Self> {
        let Some(mut right) = self.right.take() else {
            return self;
        };
        self.right = right.left.take();
        self.update_height();
        right.left = Some(self);
        right.update_height();
        right
    }

    // Rebalance after insert/delete.
    fn rebalance(mut self: Box<Self>) -> Box<Self> {
        self.update_height();

        match self.balance() {
            Ordering::Greater => {
                // Left-heavy.
                if let Some(left) = &self.left {
                    if Self::link_height(&left.left) < Self::link_height(&left.right) {
                        // LR case: left child's right side is taller — left-rotate it first.
                        if let Some(l) = self.left.take() {
                            self.left = Some(l.rotate_left());
                        }
                    }
                }
                // LL case (or LR after fix): right-rotate self.
                return self.rotate_right();
            }
            Ordering::Less => {
                // Right-heavy.
                if let Some(right) = &self.right {
                    if Self::link_height(&right.left) > Self::link_height(&right.right) {
                        // RL case: right child's left side is taller — right-rotate it first.
                        if let Some(r) = self.right.take() {
                            self.right = Some(r.rotate_right());
                        }
                    }
                }
                // RR case (or RL after fix): left-rotate self.
                return self.rotate_left();
            }
            Ordering::Equal => return self,
        }
    }

    fn insert(node: Link<K, V>, key: K, value: V) -> Link<K, V> {
        let Some(mut node) = node else {
            return Some(Box::new(Self::new(key, value)));
        };
        match key.cmp(&node.key) {
            Ordering::Equal => node.value = value,
            Ordering::Less => node.left = Self::insert(node.left.take(), key, value),
            Ordering::Greater => node.right = Self::insert(node.right.take(), key, value),
        }
        Some(node.rebalance())
    }

    fn get<'a>(node: &'a Link<K, V>, key: &K) -> Option<&'a V> {
        match node {
            None => None,
            Some(n) => match key.cmp(&n.key) {
                Ordering::Equal => Some(&n.value),
                Ordering::Less => Self::get(&n.left, key),
                Ordering::Greater => Self::get(&n.right, key),
            },
        }
    }

    fn remove(node: Link<K, V>, key: &K) -> (Link<K, V>, Option<V>) {
        let Some(mut node) = node else {
            return (None, None);
        };
        match key.cmp(&node.key) {
            Ordering::Less => {
                let (new_left, removed) = Self::remove(node.left.take(), key);
                node.left = new_left;
                (Some(node.rebalance()), removed)
            }
            Ordering::Greater => {
                let (new_right, removed) = Self::remove(node.right.take(), key);
                node.right = new_right;
                (Some(node.rebalance()), removed)
            }
            Ordering::Equal => {
                let removed_value = node.value;
                match (node.left.take(), node.right.take()) {
                    (None, right) => (right, Some(removed_value)),
                    (left, None) => (left, Some(removed_value)),
                    (left, Some(right)) => {
                        // Two children: replace with in-order successor.
                        let (mut successor, new_right) = Self::extract_min(right);
                        successor.left = left;
                        successor.right = new_right;
                        (Some(successor.rebalance()), Some(removed_value))
                    }
                }
            }
        }
    }

    /// Extract the minimum node from a non-empty subtree,
    /// returning the detached node and the remaining subtree.
    fn extract_min(mut node: Box<Node<K, V>>) -> (Box<Node<K, V>>, Link<K, V>) {
        match node.left.take() {
            None => {
                let right = node.right.take();
                (node, right)
            }
            Some(left) => {
                let (min, new_left) = Self::extract_min(left);
                node.left = new_left;
                (min, Some(node.rebalance()))
            }
        }
    }

}

/// AVL tree map — self-balancing BST with O(log n) insert, get, delete.
/// Stores key-value pairs ordered by key.
pub struct AvlTree<K, V> {
    root: Link<K, V>,
}

impl<K: Ord, V> AvlTree<K, V> {
    #[must_use]
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Insert a key-value pair. If the key already exists, updates the value. O(log n).
    pub fn insert(&mut self, key: K, value: V) {
        self.root = Node::insert(self.root.take(), key, value);
    }

    /// Get a reference to the value for a key. O(log n).
    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        Node::get(&self.root, key)
    }

    /// Remove a key, returning its value if it existed. O(log n).
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let (new_root, removed) = Node::remove(self.root.take(), key);
        self.root = new_root;
        removed
    }

    /// In-order iterator — yields `(&K, &V)` pairs sorted by key.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter::new(&self.root)
    }

    /// Max depth. O(1) — stored in root node. Used in tests to verify balancing.
    #[cfg(test)]
    fn depth(&self) -> usize {
        Node::link_height(&self.root)
    }
}

/// In-order iterator over an AVL tree. Maintains an explicit stack of
/// nodes whose left subtree has been visited but the node itself hasn't
/// been yielded yet. O(log n) stack space.
pub struct Iter<'a, K, V> {
    stack: Vec<&'a Node<K, V>>,
}

impl<'a, K, V> Iter<'a, K, V> {
    fn new(root: &'a Link<K, V>) -> Self {
        let mut iter = Self { stack: Vec::new() };
        iter.push_left_spine(root);
        iter
    }

    /// Push a node and all its left descendants onto the stack.
    fn push_left_spine(&mut self, mut link: &'a Link<K, V>) {
        while let Some(node) = link {
            self.stack.push(node);
            link = &node.left;
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        self.push_left_spine(&node.right);
        Some((&node.key, &node.value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn empty() {
        let tree = AvlTree::<i32, &str>::new();
        assert_eq!(tree.get(&1), None);
        assert!(tree.iter().next().is_none());
        assert_eq!(tree.depth(), 0);
    }

    // ONE
    #[test]
    fn single() {
        let mut tree = AvlTree::new();
        tree.insert(42, "answer");
        assert_eq!(tree.get(&42), Some(&"answer"));
        assert_eq!(tree.get(&0), None);
        assert_eq!(tree.depth(), 1);
    }

    // MANY: insert preserves BST order, get retrieves values
    #[test]
    fn iter_is_sorted() {
        let mut tree = AvlTree::new();
        for (k, v) in [
            (4, "d"),
            (2, "b"),
            (6, "f"),
            (1, "a"),
            (3, "c"),
            (5, "e"),
            (7, "g"),
        ] {
            tree.insert(k, v);
        }
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &2, &3, &4, &5, &6, &7]);
        assert_eq!(tree.get(&4), Some(&"d"));
        assert_eq!(tree.get(&7), Some(&"g"));
    }

    // Insert with existing key updates value
    #[test]
    fn insert_updates_value() {
        let mut tree = AvlTree::new();
        tree.insert(1, "old");
        tree.insert(1, "new");
        assert_eq!(tree.get(&1), Some(&"new"));
        assert_eq!(tree.iter().count(), 1);
    }

    // EDGE: inserting sorted sequence stays balanced
    #[test]
    fn sorted_insert_balanced() {
        let mut tree = AvlTree::new();
        for v in 1..=15 {
            tree.insert(v, v * 10);
        }
        assert_eq!(tree.iter().count(), 15);
        assert_eq!(tree.get(&5), Some(&50));
        assert!(tree.depth() <= 5);
    }

    // EDGE: reverse sorted insert stays balanced
    #[test]
    fn reverse_insert_balanced() {
        let mut tree = AvlTree::new();
        for v in (1..=15).rev() {
            tree.insert(v, v);
        }
        assert!(tree.depth() <= 5);
    }

    // LL rotation: insert 3, 2, 1
    #[test]
    fn ll_rotation() {
        let mut tree = AvlTree::new();
        for k in [3, 2, 1] {
            tree.insert(k, ());
        }
        // After LL rotation, root should be 2
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &2, &3]);
        assert_eq!(tree.depth(), 2);
    }

    // RR rotation: insert 1, 2, 3
    #[test]
    fn rr_rotation() {
        let mut tree = AvlTree::new();
        for k in [1, 2, 3] {
            tree.insert(k, ());
        }
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &2, &3]);
        assert_eq!(tree.depth(), 2);
    }

    // LR rotation: insert 3, 1, 2
    #[test]
    fn lr_rotation() {
        let mut tree = AvlTree::new();
        for k in [3, 1, 2] {
            tree.insert(k, ());
        }
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &2, &3]);
        assert_eq!(tree.depth(), 2);
    }

    // RL rotation: insert 1, 3, 2
    #[test]
    fn rl_rotation() {
        let mut tree = AvlTree::new();
        for k in [1, 3, 2] {
            tree.insert(k, ());
        }
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &2, &3]);
        assert_eq!(tree.depth(), 2);
    }

    // DELETE: leaf
    #[test]
    fn delete_leaf() {
        let mut tree = AvlTree::new();
        for v in [2, 1, 3] {
            tree.insert(v, v);
        }
        assert_eq!(tree.remove(&3), Some(3));
        assert_eq!(tree.get(&3), None);
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &2]);
    }

    // DELETE: node with one child
    #[test]
    fn delete_one_child() {
        let mut tree = AvlTree::new();
        for v in [3, 2, 4, 1] {
            tree.insert(v, v);
        }
        tree.remove(&2);
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &3, &4]);
    }

    // DELETE: node with two children
    #[test]
    fn delete_two_children() {
        let mut tree = AvlTree::new();
        for v in [4, 2, 6, 1, 3, 5, 7] {
            tree.insert(v, v);
        }
        tree.remove(&4);
        assert_eq!(tree.get(&4), None);
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&1, &2, &3, &5, &6, &7]);
    }

    // DELETE: triggers rebalance
    #[test]
    fn delete_rebalances() {
        let mut tree = AvlTree::new();
        for v in [4, 2, 6, 1, 3, 5, 7] {
            tree.insert(v, v);
        }
        tree.remove(&1);
        tree.remove(&3);
        tree.remove(&2);
        assert!(tree.depth() <= 3);
        let keys: Vec<&i32> = tree.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&4, &5, &6, &7]);
    }

    // DELETE: nonexistent key is a no-op
    #[test]
    fn delete_missing() {
        let mut tree = AvlTree::new();
        tree.insert(1, "a");
        assert_eq!(tree.remove(&99), None);
        assert_eq!(tree.get(&1), Some(&"a"));
    }

    // EDGE: works with String keys
    #[test]
    fn string_keys() {
        let mut tree = AvlTree::new();
        tree.insert("banana".to_string(), 2);
        tree.insert("apple".to_string(), 1);
        tree.insert("cherry".to_string(), 3);
        assert_eq!(tree.get(&"banana".to_string()), Some(&2));
        let keys: Vec<&str> = tree.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["apple", "banana", "cherry"]);
    }
}
