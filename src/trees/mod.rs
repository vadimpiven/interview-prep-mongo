// Tree data structures and algorithms.
// HR: "Know about trees; basic tree construction, traversal and manipulation."

/// AVL tree -- self-balancing BST, O(log n) guaranteed.
/// HR: "Familiarize yourself with binary trees. Be familiar with at least one balanced BST."
/// Insert, search, delete with automatic rebalancing via rotations (LL/RR/LR/RL).
pub mod avl_tree;

/// Splay tree -- self-adjusting BST, O(log n) amortized.
/// No balance invariant — every access splays the node to root.
/// Adapts to access patterns: frequently used nodes stay near root.
pub mod splay_tree;

/// Trie (prefix tree) -- insert, search, `starts_with`, `count_with_prefix`, delete.
/// HR: "Familiarize yourself with trie-trees."
/// `MongoDB`: compound index prefix matching -- index `{a,b,c}` serves `{a}`, `{a,b}`, `{a,b,c}`.
pub mod trie;

/// N-ary expression tree -- evaluate, optimize (flatten AND/OR), walk with visitor.
/// Mirrors `MongoDB`'s `MatchExpression` (`expression.h`, `expression_tree.h`, `tree_walker.h`).
/// `MongoDB` trees are NOT binary: AND/OR have N children, NOT has 1, leaves have 0.
pub mod expression_tree;
