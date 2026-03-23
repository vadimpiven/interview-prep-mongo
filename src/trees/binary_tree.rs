// Binary Tree — BST insert/search + traversals + basic manipulation
//
// Scoped for 20 min implementation + 10 min tests:
//   - TreeNode struct with Box<Option> children
//   - BST insert and search
//   - In-order traversal (sorted output proof)
//   - BFS (level-order) traversal
//   - Max depth
//
// Balanced BST discussion points (don't implement — know the concepts):
//   AVL: height-balanced, |height(left) - height(right)| <= 1, rotations on insert/delete
//   Red-black: color invariant, O(log n) guaranteed, used in std::map
//   B-tree: used by WiredTiger for indexes, high fan-out, disk-friendly
//   "If asked, I'd add left/right rotations to maintain balance on insert."

use std::collections::VecDeque;

type Link = Option<Box<TreeNode>>;

pub struct TreeNode {
    pub val: i64,
    pub left: Link,
    pub right: Link,
}

impl TreeNode {
    /// O(1).
    #[must_use]
    pub fn new(val: i64) -> Self {
        Self {
            val,
            left: None,
            right: None,
        }
    }
}

/// BST insert. O(log n) average, O(n) worst case (sorted input -> degenerate).
pub fn bst_insert(root: &mut Link, val: i64) {
    match root {
        None => *root = Some(Box::new(TreeNode::new(val))),
        Some(node) => {
            if val < node.val {
                bst_insert(&mut node.left, val);
            } else {
                bst_insert(&mut node.right, val);
            }
        }
    }
}

/// BST search. O(log n) average.
#[must_use]
pub fn bst_search(root: &Link, val: i64) -> bool {
    match root {
        None => false,
        Some(node) if val == node.val => true,
        Some(node) if val < node.val => bst_search(&node.left, val),
        Some(node) => bst_search(&node.right, val),
    }
}

/// In-order DFS: left -> root -> right. Produces sorted output for BST. O(n).
#[must_use]
pub fn inorder(root: &Link) -> Vec<i64> {
    fn walk(node: &Link, result: &mut Vec<i64>) {
        if let Some(n) = node {
            walk(&n.left, result);
            result.push(n.val);
            walk(&n.right, result);
        }
    }

    let mut result = Vec::new();
    walk(root, &mut result);
    result
}

/// BFS / level-order traversal. O(n).
#[must_use]
pub fn bfs(root: &Link) -> Vec<i64> {
    let mut result = Vec::new();
    let mut queue: VecDeque<&TreeNode> = VecDeque::new();
    if let Some(node) = root {
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

/// Max depth. O(n).
#[must_use]
pub fn max_depth(root: &Link) -> usize {
    match root {
        None => 0,
        Some(node) => 1 + max_depth(&node.left).max(max_depth(&node.right)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_bst() -> Link {
        //       4
        //      / \
        //     2   6
        //    / \ / \
        //   1  3 5  7
        let mut root = None;
        for v in [4, 2, 6, 1, 3, 5, 7] {
            bst_insert(&mut root, v);
        }
        root
    }

    // ZERO
    #[test]
    fn empty_tree() {
        let root: Link = None;
        assert!(!bst_search(&root, 1));
        assert_eq!(inorder(&root), vec![]);
        assert_eq!(bfs(&root), vec![]);
        assert_eq!(max_depth(&root), 0);
    }

    // ONE
    #[test]
    fn single_node() {
        let mut root = None;
        bst_insert(&mut root, 42);
        assert!(bst_search(&root, 42));
        assert_eq!(max_depth(&root), 1);
    }

    // MANY: insert, search, traversals
    #[test]
    fn insert_and_search() {
        let root = build_bst();
        assert!(bst_search(&root, 4));
        assert!(bst_search(&root, 1));
        assert!(bst_search(&root, 7));
        assert!(!bst_search(&root, 0));
        assert!(!bst_search(&root, 8));
    }

    #[test]
    fn inorder_is_sorted() {
        let root = build_bst();
        assert_eq!(inorder(&root), vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn bfs_level_order() {
        let root = build_bst();
        assert_eq!(bfs(&root), vec![4, 2, 6, 1, 3, 5, 7]);
    }

    // EDGE: depth
    #[test]
    fn depth() {
        let root = build_bst();
        assert_eq!(max_depth(&root), 3);
    }
}
