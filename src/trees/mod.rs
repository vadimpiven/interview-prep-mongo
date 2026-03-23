// Tree data structures and algorithms.
// HR: "Know about trees; basic tree construction, traversal and manipulation."

/// Binary tree -- construction, BFS/DFS traversal, manipulation, BST insert/search.
/// HR: "Familiarize yourself with binary trees. Be familiar with at least one balanced BST."
/// Balanced BST concepts (AVL/red-black) covered in comments for discussion.
pub mod binary_tree;

/// Trie (prefix tree) -- insert, search, `starts_with`, `count_with_prefix`, delete.
/// HR: "Familiarize yourself with trie-trees."
/// `MongoDB`: compound index prefix matching -- index `{a,b,c}` serves `{a}`, `{a,b}`, `{a,b,c}`.
pub mod trie;

/// N-ary expression tree -- evaluate, optimize (flatten AND/OR), walk with visitor.
/// Mirrors `MongoDB`'s `MatchExpression` (`expression.h`, `expression_tree.h`, `tree_walker.h`).
/// `MongoDB` trees are NOT binary: AND/OR have N children, NOT has 1, leaves have 0.
pub mod expression_tree;
