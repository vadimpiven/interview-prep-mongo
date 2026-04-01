// Splay Tree — Self-adjusting BST, O(log n) amortized
//
// No balance invariant — instead, every access (search, insert, delete)
// splays the accessed node to the root via rotations. Frequently accessed
// nodes stay near the root, giving excellent cache/locality for skewed
// access patterns.
//
// Splay operation uses three rotation patterns:
//   Zig       — node is child of root (single rotation)
//   Zig-zig   — node and parent are both left (or both right) children
//               (rotate parent first, then node — NOT two single rotations)
//   Zig-zag   — node is left child of right parent or vice versa
//               (same as AVL double rotation)
//
// Amortized O(log n) per operation. Individual ops can be O(n) on a
// degenerate tree, but the splay restructures it for future ops.
//
// Trade-offs:
//   + No extra storage per node (no height/color field)
//   + Simplest balanced BST to implement
//   + Adapts to access patterns — working set stays near root
//   - Mutates tree on reads (search splays)
//   - No worst-case per-operation guarantee
//
// Used in: caches, garbage collectors, network routers, Windows NT kernel.

use std::cmp::Ordering;
use std::collections::VecDeque;

type Link = Option<Box<Node>>;

struct Node {
    val: i64,
    left: Link,
    right: Link,
}

impl Node {
    fn new(val: i64) -> Self {
        Self {
            val,
            left: None,
            right: None,
        }
    }
}

fn rotate_right(mut node: Box<Node>) -> Box<Node> {
    let mut left = node.left.take().unwrap();
    node.left = left.right.take();
    left.right = Some(node);
    left
}

fn rotate_left(mut node: Box<Node>) -> Box<Node> {
    let mut right = node.right.take().unwrap();
    node.right = right.left.take();
    right.left = Some(node);
    right
}

/// Splay `val` to the root. If `val` is not in the tree, the last
/// accessed node (closest value) is splayed instead.
///
/// Uses top-down splaying: split the tree into left and right subtrees
/// as we walk down, then reassemble at the end. This avoids recursion
/// and parent pointers.
fn splay(node: Link, val: i64) -> Link {
    let mut node = match node {
        None => return None,
        Some(n) => n,
    };

    match val.cmp(&node.val) {
        Ordering::Equal => Some(node),

        Ordering::Less => {
            let left = match node.left.take() {
                None => return Some(node),
                Some(l) => l,
            };

            match val.cmp(&left.val) {
                Ordering::Equal => {
                    // Zig: val is left child of root.
                    node.left = Some(left);
                    Some(rotate_right(node))
                }
                Ordering::Less => {
                    // Zig-zig: val is in left-left subtree.
                    node.left = Some(left);
                    let mut node = rotate_right(node);
                    if node.left.is_some() {
                        node.left = splay(node.left.take(), val);
                        Some(rotate_right(node))
                    } else {
                        Some(node)
                    }
                }
                Ordering::Greater => {
                    // Zig-zag: val is in left-right subtree.
                    node.left = Some(left);
                    // Splay within left subtree, then rotate up.
                    node.left = splay(node.left.take(), val);
                    Some(rotate_right(node))
                }
            }
        }

        Ordering::Greater => {
            let right = match node.right.take() {
                None => return Some(node),
                Some(r) => r,
            };

            match val.cmp(&right.val) {
                Ordering::Equal => {
                    // Zig: val is right child of root.
                    node.right = Some(right);
                    Some(rotate_left(node))
                }
                Ordering::Greater => {
                    // Zig-zig: val is in right-right subtree.
                    node.right = Some(right);
                    let mut node = rotate_left(node);
                    if node.right.is_some() {
                        node.right = splay(node.right.take(), val);
                        Some(rotate_left(node))
                    } else {
                        Some(node)
                    }
                }
                Ordering::Less => {
                    // Zig-zag: val is in right-left subtree.
                    node.right = Some(right);
                    node.right = splay(node.right.take(), val);
                    Some(rotate_left(node))
                }
            }
        }
    }
}

// --- Public API ---

/// Splay tree — self-adjusting BST with O(log n) amortized operations.
pub struct SplayTree {
    root: Link,
}

impl SplayTree {
    #[must_use]
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Insert a value. O(log n) amortized.
    pub fn insert(&mut self, val: i64) {
        self.root = splay(self.root.take(), val);

        let root = match self.root.take() {
            None => {
                self.root = Some(Box::new(Node::new(val)));
                return;
            }
            Some(r) => r,
        };

        let mut root = root;
        let mut new_node = Box::new(Node::new(val));
        match val.cmp(&root.val) {
            Ordering::Equal => {
                // Duplicate: insert to the right.
                new_node.left = Some(root);
            }
            Ordering::Less => {
                // New node takes root's left subtree; root becomes right child.
                new_node.left = root.left.take();
                new_node.right = Some(root);
            }
            Ordering::Greater => {
                // New node takes root's right subtree; root becomes left child.
                new_node.right = root.right.take();
                new_node.left = Some(root);
            }
        }
        self.root = Some(new_node);
    }

    /// Search for a value. O(log n) amortized.
    /// Splays the found node (or closest node) to root.
    #[must_use]
    pub fn search(&mut self, val: i64) -> bool {
        self.root = splay(self.root.take(), val);
        self.root.as_ref().is_some_and(|n| n.val == val)
    }

    /// Delete a value. O(log n) amortized.
    pub fn delete(&mut self, val: i64) {
        self.root = splay(self.root.take(), val);

        let root = match self.root.take() {
            None => return,
            Some(r) => r,
        };

        if root.val != val {
            self.root = Some(root);
            return;
        }

        // Root is the node to delete. Merge left and right subtrees.
        match (root.left, root.right) {
            (None, right) => self.root = right,
            (left, None) => self.root = left,
            (left, right) => {
                // Splay max of left subtree to its root, then attach right.
                let mut new_root = splay(left, i64::MAX).unwrap();
                new_root.right = right;
                self.root = Some(new_root);
            }
        }
    }

    /// In-order traversal (sorted output). O(n).
    #[must_use]
    pub fn inorder(&self) -> Vec<i64> {
        let mut result = Vec::new();
        Self::inorder_walk(&self.root, &mut result);
        result
    }

    /// BFS / level-order traversal. O(n).
    #[must_use]
    pub fn bfs(&self) -> Vec<i64> {
        let mut result = Vec::new();
        let mut queue: VecDeque<&Node> = VecDeque::new();
        if let Some(node) = &self.root {
            queue.push_back(node);
        }
        while let Some(node) = queue.pop_front() {
            result.push(node.val);
            if let Some(ref left) = node.left {
                queue.push_back(left);
            }
            if let Some(ref right) = node.right {
                queue.push_back(right);
            }
        }
        result
    }

    fn inorder_walk(node: &Link, result: &mut Vec<i64>) {
        if let Some(n) = node {
            Self::inorder_walk(&n.left, result);
            result.push(n.val);
            Self::inorder_walk(&n.right, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ZERO
    #[test]
    fn empty() {
        let mut tree = SplayTree::new();
        assert!(!tree.search(1));
        assert_eq!(tree.inorder(), vec![]);
        assert_eq!(tree.bfs(), vec![]);
    }

    // ONE
    #[test]
    fn single() {
        let mut tree = SplayTree::new();
        tree.insert(42);
        assert!(tree.search(42));
        assert!(!tree.search(0));
    }

    // MANY: insert preserves BST order
    #[test]
    fn inorder_is_sorted() {
        let mut tree = SplayTree::new();
        for v in [4, 2, 6, 1, 3, 5, 7] {
            tree.insert(v);
        }
        assert_eq!(tree.inorder(), vec![1, 2, 3, 4, 5, 6, 7]);
    }

    // Search splays to root
    #[test]
    fn search_splays_to_root() {
        let mut tree = SplayTree::new();
        for v in [4, 2, 6, 1, 3, 5, 7] {
            tree.insert(v);
        }
        let _ = tree.search(1);
        assert_eq!(tree.root.as_ref().unwrap().val, 1);
    }

    // EDGE: sorted insert (worst case for naive BST)
    #[test]
    fn sorted_insert() {
        let mut tree = SplayTree::new();
        for v in 1..=15 {
            tree.insert(v);
        }
        assert_eq!(tree.inorder(), (1..=15).collect::<Vec<_>>());
    }

    // DELETE: leaf
    #[test]
    fn delete_leaf() {
        let mut tree = SplayTree::new();
        for v in [2, 1, 3] {
            tree.insert(v);
        }
        tree.delete(3);
        assert!(!tree.search(3));
        let inorder = tree.inorder();
        assert!(inorder.contains(&1));
        assert!(inorder.contains(&2));
    }

    // DELETE: node with two children
    #[test]
    fn delete_root() {
        let mut tree = SplayTree::new();
        for v in [4, 2, 6, 1, 3, 5, 7] {
            tree.insert(v);
        }
        tree.delete(4);
        assert!(!tree.search(4));
        assert_eq!(tree.inorder(), vec![1, 2, 3, 5, 6, 7]);
    }

    // DELETE: nonexistent value is a no-op
    #[test]
    fn delete_missing() {
        let mut tree = SplayTree::new();
        tree.insert(1);
        tree.delete(99);
        assert_eq!(tree.inorder(), vec![1]);
    }

    // EDGE: duplicates
    #[test]
    fn duplicates() {
        let mut tree = SplayTree::new();
        tree.insert(1);
        tree.insert(1);
        tree.insert(1);
        assert_eq!(tree.inorder(), vec![1, 1, 1]);
    }

    // Repeated access pattern — splay adapts
    #[test]
    fn repeated_access() {
        let mut tree = SplayTree::new();
        for v in 1..=100 {
            tree.insert(v);
        }
        // Accessing the same element repeatedly should keep it at root
        for _ in 0..10 {
            assert!(tree.search(50));
            assert_eq!(tree.root.as_ref().unwrap().val, 50);
        }
    }
}
