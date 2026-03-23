// N-ary Expression Tree — MongoDB's MatchExpression pattern
//
// Scoped for 20 min implementation + 10 min tests:
//   - Expr struct with ExprType enum and Vec<Box<Expr>> children
//   - Constructors: and_expr, or_expr, eq, gt
//   - evaluate() against a document (HashMap<String, i64>)
//   - optimize() — flatten nested AND/OR, eliminate AlwaysTrue/AlwaysFalse
//
// NOT included (mention in discussion if asked):
//   - Visitor/walk pattern (describe: preVisit/inVisit/postVisit from tree_walker.h)
//   - clone() (trivial: recurse children)
//   - NOT node, de Morgan transforms

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprType {
    And,
    Or,
    Eq,
    Gt,
    AlwaysTrue,
    AlwaysFalse,
}

pub struct Expr {
    pub expr_type: ExprType,
    pub children: Vec<Box<Expr>>,
    pub field: Option<String>,
    pub value: Option<i64>,
}

impl Expr {
    /// Create an AND node with N children. O(1).
    #[must_use]
    pub fn and_expr(children: Vec<Box<Expr>>) -> Box<Expr> {
        Box::new(Expr {
            expr_type: ExprType::And,
            children,
            field: None,
            value: None,
        })
    }

    /// Create an OR node with N children. O(1).
    #[must_use]
    pub fn or_expr(children: Vec<Box<Expr>>) -> Box<Expr> {
        Box::new(Expr {
            expr_type: ExprType::Or,
            children,
            field: None,
            value: None,
        })
    }

    /// Create an equality leaf: field == value. O(1).
    #[must_use]
    pub fn eq(field: &str, value: i64) -> Box<Expr> {
        Box::new(Expr {
            expr_type: ExprType::Eq,
            children: vec![],
            field: Some(field.to_string()),
            value: Some(value),
        })
    }

    /// Create a greater-than leaf: field > value. O(1).
    #[must_use]
    pub fn gt(field: &str, value: i64) -> Box<Expr> {
        Box::new(Expr {
            expr_type: ExprType::Gt,
            children: vec![],
            field: Some(field.to_string()),
            value: Some(value),
        })
    }

    /// Leaf that always matches. O(1).
    #[must_use]
    pub fn always_true() -> Box<Expr> {
        Box::new(Expr {
            expr_type: ExprType::AlwaysTrue,
            children: vec![],
            field: None,
            value: None,
        })
    }

    /// Leaf that never matches. O(1).
    #[must_use]
    pub fn always_false() -> Box<Expr> {
        Box::new(Expr {
            expr_type: ExprType::AlwaysFalse,
            children: vec![],
            field: None,
            value: None,
        })
    }

    /// Number of direct children. O(1).
    #[must_use]
    pub fn num_children(&self) -> usize {
        self.children.len()
    }
}

pub type Document = HashMap<String, i64>;

/// Evaluate expression against a document. O(n) where n = tree nodes.
/// Mirrors `MatchExpression::matches()`.
///
/// # Panics
///
/// Panics if an `Eq` or `Gt` node is missing its `field` or `value`.
#[must_use]
pub fn evaluate(expr: &Expr, doc: &Document) -> bool {
    match expr.expr_type {
        ExprType::And => expr.children.iter().all(|c| evaluate(c, doc)),
        ExprType::Or => expr.children.iter().any(|c| evaluate(c, doc)),
        ExprType::Eq => doc.get(expr.field.as_ref().unwrap()) == Some(&expr.value.unwrap()),
        ExprType::Gt => doc
            .get(expr.field.as_ref().unwrap())
            .is_some_and(|v| *v > expr.value.unwrap()),
        ExprType::AlwaysTrue => true,
        ExprType::AlwaysFalse => false,
    }
}

/// Simplify expression tree. O(n) where n = tree nodes.
/// Mirrors `MatchExpression::optimize()`:
///   1. Flatten nested `AND(AND(a,b), c)` -> `AND(a, b, c)`. Same for OR.
///   2. Single-child AND/OR -> unwrap
///   3. AND with `AlwaysFalse` -> `AlwaysFalse`. OR with `AlwaysTrue` -> `AlwaysTrue`.
///   4. Remove `AlwaysTrue` from AND, `AlwaysFalse` from OR.
///
/// # Panics
///
/// Will not panic in practice: the `unwrap` on `flat.pop()` is guarded by
/// the `match` arm requiring `flat.len() == 1`.
// Boxing is inherent to the recursive tree structure — Expr children are Vec<Box<Expr>>.
#[allow(clippy::boxed_local)]
#[must_use]
pub fn optimize(expr: Box<Expr>) -> Box<Expr> {
    let expr = *expr;
    let children: Vec<Box<Expr>> = expr.children.into_iter().map(optimize).collect();

    match expr.expr_type {
        ExprType::And => {
            let mut flat = Vec::new();
            for c in children {
                if c.expr_type == ExprType::And {
                    flat.extend(c.children);
                } else {
                    flat.push(c);
                }
            }
            if flat.iter().any(|c| c.expr_type == ExprType::AlwaysFalse) {
                return Expr::always_false();
            }
            flat.retain(|c| c.expr_type != ExprType::AlwaysTrue);
            match flat.len() {
                0 => Expr::always_true(),
                1 => flat.pop().unwrap(),
                _ => Expr::and_expr(flat),
            }
        }
        ExprType::Or => {
            let mut flat = Vec::new();
            for c in children {
                if c.expr_type == ExprType::Or {
                    flat.extend(c.children);
                } else {
                    flat.push(c);
                }
            }
            if flat.iter().any(|c| c.expr_type == ExprType::AlwaysTrue) {
                return Expr::always_true();
            }
            flat.retain(|c| c.expr_type != ExprType::AlwaysFalse);
            match flat.len() {
                0 => Expr::always_false(),
                1 => flat.pop().unwrap(),
                _ => Expr::or_expr(flat),
            }
        }
        _ => Box::new(Expr {
            expr_type: expr.expr_type,
            children,
            field: expr.field,
            value: expr.value,
        }),
    }
}

/// Count total nodes in tree. O(n).
#[must_use]
pub fn count_nodes(expr: &Expr) -> usize {
    1 + expr.children.iter().map(|c| count_nodes(c)).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(pairs: &[(&str, i64)]) -> Document {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    // ONE: single leaf
    #[test]
    fn eval_missing_field() {
        let expr = Expr::eq("a", 5);
        assert!(!evaluate(&expr, &doc(&[("b", 5)])));
    }

    // MANY: AND and OR evaluation
    #[test]
    fn eval_and_or() {
        let and = Expr::and_expr(vec![Expr::eq("a", 5), Expr::gt("b", 3)]);
        assert!(evaluate(&and, &doc(&[("a", 5), ("b", 10)])));
        assert!(!evaluate(&and, &doc(&[("a", 5), ("b", 1)])));

        let or = Expr::or_expr(vec![Expr::eq("a", 5), Expr::eq("b", 10)]);
        assert!(evaluate(&or, &doc(&[("a", 5)])));
        assert!(!evaluate(&or, &doc(&[("a", 6), ("b", 9)])));
    }

    // EDGE: flatten nested AND
    #[test]
    fn optimize_flatten_and() {
        let expr = Expr::and_expr(vec![
            Expr::and_expr(vec![Expr::eq("a", 1), Expr::eq("b", 2)]),
            Expr::eq("c", 3),
        ]);
        let opt = optimize(expr);
        assert_eq!(opt.num_children(), 3);
        assert_eq!(count_nodes(&opt), 4);
    }

    // EDGE: short-circuit
    #[test]
    fn optimize_and_false_short_circuit() {
        let expr = Expr::and_expr(vec![Expr::eq("a", 1), Expr::always_false()]);
        assert_eq!(optimize(expr).expr_type, ExprType::AlwaysFalse);
    }

    // EDGE: unwrap single child
    #[test]
    fn optimize_unwrap_single_child() {
        let expr = Expr::and_expr(vec![Expr::eq("a", 1)]);
        assert_eq!(optimize(expr).expr_type, ExprType::Eq);
    }

    // EDGE: optimize preserves evaluation semantics
    #[test]
    fn optimize_preserves_semantics() {
        let expr = Expr::and_expr(vec![
            Expr::and_expr(vec![Expr::eq("a", 1), Expr::always_true()]),
            Expr::or_expr(vec![Expr::always_false(), Expr::gt("b", 5)]),
        ]);
        let d = doc(&[("a", 1), ("b", 10)]);
        let before = evaluate(&expr, &d);
        let opt = optimize(expr);
        assert_eq!(evaluate(&opt, &d), before);
        assert_eq!(count_nodes(&opt), 3);
    }
}
