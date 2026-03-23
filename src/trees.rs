// Trees — BST, AVL Tree, Traversals
//
// "Know about trees; basic tree construction, traversal and manipulation
// algorithms. Familiarize yourself with binary trees, n-ary trees, and
// trie-trees. Be familiar with at least one type of balanced binary tree,
// whether it's a red/black tree, a splay tree or an AVL tree, and know
// how it's implemented."
//
// This module implements:
//   1. BST — unbalanced binary search tree with insert, contains, traversals
//   2. AVL tree — self-balancing BST (height-balanced, rotations on insert)
//
// BST complexity:
//   insert/contains: O(h) where h = height. O(log n) average, O(n) worst (degenerate).
//   Traversals: O(n) time, O(h) stack space.
//
// AVL complexity:
//   insert/contains: O(log n) guaranteed. Height is always ≤ 1.44 * log2(n).
//   Rotations restore balance after every insert in O(1) per rotation.
//
// MongoDB uses B-trees for on-disk indexes (WiredTiger), not AVL trees.
// But AVL trees demonstrate the same balancing principles (rotations, height
// invariants) that apply to all balanced tree variants.

use std::cmp::Ordering;
use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// BST — Unbalanced Binary Search Tree
// ---------------------------------------------------------------------------

/// Simple unbalanced BST. Demonstrates construction, lookup, and traversals.
/// insert/contains: O(log n) average, O(n) worst case (sorted input).
pub struct BST<T> {
    root: Link<T>,
}

type Link<T> = Option<Box<BSTNode<T>>>;

struct BSTNode<T> {
    value: T,
    left: Link<T>,
    right: Link<T>,
}

impl<T: Ord> BST<T> {
    /// O(1).
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Insert a value. Duplicates are ignored.
    /// O(h) where h = tree height.
    pub fn insert(&mut self, value: T) {
        Self::insert_into(&mut self.root, value);
    }

    /// Check if value exists in the tree.
    /// O(h) where h = tree height.
    #[must_use]
    pub fn contains(&self, value: &T) -> bool {
        Self::find(&self.root, value)
    }

    /// In-order traversal (left, root, right). Produces sorted output.
    /// O(n) time, O(h) stack space.
    pub fn in_order(&self) -> Vec<&T> {
        let mut result = Vec::new();
        Self::in_order_walk(&self.root, &mut result);
        result
    }

    /// Pre-order traversal (root, left, right). Useful for serialization.
    /// O(n) time, O(h) stack space.
    pub fn pre_order(&self) -> Vec<&T> {
        let mut result = Vec::new();
        Self::pre_order_walk(&self.root, &mut result);
        result
    }

    /// Level-order (BFS) traversal. Visits nodes top-to-bottom, left-to-right.
    /// O(n) time, O(w) space where w = max width of the tree.
    pub fn level_order(&self) -> Vec<&T> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        if let Some(ref node) = self.root {
            queue.push_back(node.as_ref());
        }
        while let Some(node) = queue.pop_front() {
            result.push(&node.value);
            if let Some(ref left) = node.left {
                queue.push_back(left.as_ref());
            }
            if let Some(ref right) = node.right {
                queue.push_back(right.as_ref());
            }
        }
        result
    }

    fn insert_into(link: &mut Link<T>, value: T) {
        match link {
            None => {
                *link = Some(Box::new(BSTNode {
                    value,
                    left: None,
                    right: None,
                }));
            }
            Some(node) => match value.cmp(&node.value) {
                Ordering::Less => Self::insert_into(&mut node.left, value),
                Ordering::Greater => Self::insert_into(&mut node.right, value),
                Ordering::Equal => {} // duplicate, ignore
            },
        }
    }

    fn find(link: &Link<T>, value: &T) -> bool {
        match link {
            None => false,
            Some(node) => match value.cmp(&node.value) {
                Ordering::Equal => true,
                Ordering::Less => Self::find(&node.left, value),
                Ordering::Greater => Self::find(&node.right, value),
            },
        }
    }

    fn in_order_walk<'a>(link: &'a Link<T>, result: &mut Vec<&'a T>) {
        if let Some(node) = link {
            Self::in_order_walk(&node.left, result);
            result.push(&node.value);
            Self::in_order_walk(&node.right, result);
        }
    }

    fn pre_order_walk<'a>(link: &'a Link<T>, result: &mut Vec<&'a T>) {
        if let Some(node) = link {
            result.push(&node.value);
            Self::pre_order_walk(&node.left, result);
            Self::pre_order_walk(&node.right, result);
        }
    }
}

// ---------------------------------------------------------------------------
// AVL Tree — Self-Balancing BST
// ---------------------------------------------------------------------------

/// AVL tree: height-balanced BST where |height(left) - height(right)| ≤ 1
/// for every node. Rebalances via rotations after each insert.
/// insert/contains: O(log n) guaranteed.
pub struct AVLTree<T> {
    root: AVLLink<T>,
}

type AVLLink<T> = Option<Box<AVLNode<T>>>;

struct AVLNode<T> {
    value: T,
    left: AVLLink<T>,
    right: AVLLink<T>,
    height: i32,
}

impl<T: Ord> AVLTree<T> {
    /// O(1).
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Insert a value. Rebalances the tree if needed.
    /// O(log n) — traversal down + at most O(log n) rotations back up.
    pub fn insert(&mut self, value: T) {
        self.root = Self::insert_into(self.root.take(), value);
    }

    /// Check if value exists in the tree.
    /// O(log n) guaranteed due to height balance.
    #[must_use]
    pub fn contains(&self, value: &T) -> bool {
        Self::find(&self.root, value)
    }

    /// In-order traversal. Produces sorted output.
    /// O(n) time, O(log n) stack space.
    pub fn in_order(&self) -> Vec<&T> {
        let mut result = Vec::new();
        Self::in_order_walk(&self.root, &mut result);
        result
    }

    fn insert_into(link: AVLLink<T>, value: T) -> AVLLink<T> {
        let mut node = match link {
            None => {
                return Some(Box::new(AVLNode {
                    value,
                    left: None,
                    right: None,
                    height: 1,
                }));
            }
            Some(node) => node,
        };

        match value.cmp(&node.value) {
            Ordering::Less => node.left = Self::insert_into(node.left.take(), value),
            Ordering::Greater => node.right = Self::insert_into(node.right.take(), value),
            Ordering::Equal => return Some(node), // duplicate, ignore
        }

        node.height = 1 + Self::height(&node.left).max(Self::height(&node.right));
        Some(Self::rebalance(node))
    }

    /// Rebalance a node based on its balance factor.
    /// Four cases: left-left, left-right, right-right, right-left.
    fn rebalance(mut node: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
        let balance = Self::balance_factor(&node);

        // Left-heavy
        if balance > 1 {
            // Left-right case: rotate left child left, then rotate node right
            if Self::balance_factor(node.left.as_ref().unwrap()) < 0 {
                node.left = Some(Self::rotate_left(node.left.take().unwrap()));
            }
            return Self::rotate_right(node);
        }

        // Right-heavy
        if balance < -1 {
            // Right-left case: rotate right child right, then rotate node left
            if Self::balance_factor(node.right.as_ref().unwrap()) > 0 {
                node.right = Some(Self::rotate_right(node.right.take().unwrap()));
            }
            return Self::rotate_left(node);
        }

        node // already balanced
    }

    /// Right rotation: node's left child becomes new root.
    ///     node          left
    ///    /    \   →    /    \
    ///  left    C      A    node
    ///  / \                 / \
    /// A   B               B   C
    fn rotate_right(mut node: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
        let mut new_root = node.left.take().unwrap();
        node.left = new_root.right.take();
        node.height = 1 + Self::height(&node.left).max(Self::height(&node.right));
        new_root.right = Some(node);
        new_root.height = 1 + Self::height(&new_root.left).max(Self::height(&new_root.right));
        new_root
    }

    /// Left rotation: node's right child becomes new root. Mirror of rotate_right.
    fn rotate_left(mut node: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
        let mut new_root = node.right.take().unwrap();
        node.right = new_root.left.take();
        node.height = 1 + Self::height(&node.left).max(Self::height(&node.right));
        new_root.left = Some(node);
        new_root.height = 1 + Self::height(&new_root.left).max(Self::height(&new_root.right));
        new_root
    }

    fn height(link: &AVLLink<T>) -> i32 {
        link.as_ref().map_or(0, |n| n.height)
    }

    fn balance_factor(node: &AVLNode<T>) -> i32 {
        Self::height(&node.left) - Self::height(&node.right)
    }

    fn find(link: &AVLLink<T>, value: &T) -> bool {
        match link {
            None => false,
            Some(node) => match value.cmp(&node.value) {
                Ordering::Equal => true,
                Ordering::Less => Self::find(&node.left, value),
                Ordering::Greater => Self::find(&node.right, value),
            },
        }
    }

    fn in_order_walk<'a>(link: &'a AVLLink<T>, result: &mut Vec<&'a T>) {
        if let Some(node) = link {
            Self::in_order_walk(&node.left, result);
            result.push(&node.value);
            Self::in_order_walk(&node.right, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- BST tests --

    #[test]
    fn bst_insert_and_contains() {
        let mut tree = BST::new();
        tree.insert(5);
        tree.insert(3);
        tree.insert(7);
        tree.insert(1);
        tree.insert(4);
        assert!(tree.contains(&5));
        assert!(tree.contains(&1));
        assert!(!tree.contains(&99));
    }

    #[test]
    fn bst_in_order_is_sorted() {
        let mut tree = BST::new();
        for v in [5, 3, 7, 1, 4, 6, 8] {
            tree.insert(v);
        }
        assert_eq!(tree.in_order(), vec![&1, &3, &4, &5, &6, &7, &8]);
    }

    #[test]
    fn bst_pre_order() {
        let mut tree = BST::new();
        for v in [5, 3, 7, 1, 4] {
            tree.insert(v);
        }
        // Root first, then left subtree, then right subtree
        assert_eq!(tree.pre_order(), vec![&5, &3, &1, &4, &7]);
    }

    #[test]
    fn bst_level_order() {
        let mut tree = BST::new();
        for v in [5, 3, 7, 1, 4, 6, 8] {
            tree.insert(v);
        }
        // BFS: level 0 = [5], level 1 = [3,7], level 2 = [1,4,6,8]
        assert_eq!(tree.level_order(), vec![&5, &3, &7, &1, &4, &6, &8]);
    }

    // -- AVL tree tests --

    #[test]
    fn avl_insert_and_contains() {
        let mut tree = AVLTree::new();
        for v in [5, 3, 7, 1, 4, 6, 8] {
            tree.insert(v);
        }
        assert!(tree.contains(&5));
        assert!(tree.contains(&1));
        assert!(!tree.contains(&99));
    }

    #[test]
    fn avl_sorted_input_stays_balanced() {
        // Sorted input is the worst case for an unbalanced BST (degenerates to
        // a linked list). AVL must rebalance via rotations.
        let mut tree = AVLTree::new();
        for v in 1..=15 {
            tree.insert(v);
        }
        // In-order must still be sorted
        let expected: Vec<i32> = (1..=15).collect();
        let expected_refs: Vec<&i32> = expected.iter().collect();
        assert_eq!(tree.in_order(), expected_refs);

        // All elements must be findable (proves tree is correctly linked)
        for v in 1..=15 {
            assert!(tree.contains(&v), "Missing {}", v);
        }
    }

    #[test]
    fn avl_all_rotation_cases() {
        let mut tree = AVLTree::new();

        // Right-right case: insert 1, 2, 3 → triggers left rotation at 1
        tree.insert(1);
        tree.insert(2);
        tree.insert(3);
        assert_eq!(tree.in_order(), vec![&1, &2, &3]);

        // Left-left case: insert values that trigger right rotation
        let mut tree2 = AVLTree::new();
        tree2.insert(3);
        tree2.insert(2);
        tree2.insert(1);
        assert_eq!(tree2.in_order(), vec![&1, &2, &3]);

        // Left-right case: 3, 1, 2
        let mut tree3 = AVLTree::new();
        tree3.insert(3);
        tree3.insert(1);
        tree3.insert(2);
        assert_eq!(tree3.in_order(), vec![&1, &2, &3]);

        // Right-left case: 1, 3, 2
        let mut tree4 = AVLTree::new();
        tree4.insert(1);
        tree4.insert(3);
        tree4.insert(2);
        assert_eq!(tree4.in_order(), vec![&1, &2, &3]);
    }
}
