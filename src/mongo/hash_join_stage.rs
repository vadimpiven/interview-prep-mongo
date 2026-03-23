// HashJoinStage — build/probe equi-join
//
// Scoped for 20 min: implement struct + Stage trait.
// open(): drain inner child into hash table (build phase).
// getNext(): for each outer row, probe hash table (probe phase).
// Real SBE: src/mongo/db/exec/sbe/stages/hash_join.cpp
//
// Encoding: join_key = row / 1000, payload = row % 1000.
// Output: outer_payload * 1000 + inner_payload.

use super::stages::{PlanState, Stage};
use std::collections::HashMap;

pub struct HashJoinStage {
    outer: Box<dyn Stage>,
    inner: Box<dyn Stage>,
    outer_key_fn: fn(i64) -> i64,
    inner_key_fn: fn(i64) -> i64,
    outer_val_fn: fn(i64) -> i64,
    inner_val_fn: fn(i64) -> i64,
    table: HashMap<i64, Vec<i64>>,
    match_buf: Vec<i64>,
    match_idx: usize,
    outer_val: i64,
    is_eof: bool,
}

impl HashJoinStage {
    /// O(1).
    pub fn new(
        outer: Box<dyn Stage>,
        inner: Box<dyn Stage>,
        outer_key_fn: fn(i64) -> i64,
        inner_key_fn: fn(i64) -> i64,
        outer_val_fn: fn(i64) -> i64,
        inner_val_fn: fn(i64) -> i64,
    ) -> Self {
        Self {
            outer,
            inner,
            outer_key_fn,
            inner_key_fn,
            outer_val_fn,
            inner_val_fn,
            table: HashMap::new(),
            match_buf: Vec::new(),
            match_idx: 0,
            outer_val: 0,
            is_eof: false,
        }
    }
}

impl Stage for HashJoinStage {
    fn open(&mut self, re_open: bool) {
        // Build phase: materialize inner side
        self.inner.open(re_open);
        self.table.clear();
        while let (PlanState::Advanced, Some(row)) = self.inner.get_next() {
            self.table
                .entry((self.inner_key_fn)(row))
                .or_default()
                .push((self.inner_val_fn)(row));
        }
        self.inner.close();
        self.outer.open(re_open);
        self.match_buf.clear();
        self.match_idx = 0;
        self.is_eof = false;
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        loop {
            // Drain buffered matches from current outer row
            if self.match_idx < self.match_buf.len() {
                let inner_val = self.match_buf[self.match_idx];
                self.match_idx += 1;
                return (PlanState::Advanced, Some(self.outer_val * 1000 + inner_val));
            }
            if self.is_eof {
                return (PlanState::Eof, None);
            }

            // Pull next outer row, probe
            match self.outer.get_next() {
                (PlanState::Eof, _) => {
                    self.is_eof = true;
                    return (PlanState::Eof, None);
                }
                (PlanState::Advanced, Some(row)) => {
                    let key = (self.outer_key_fn)(row);
                    self.outer_val = (self.outer_val_fn)(row);
                    self.match_buf = self.table.get(&key).cloned().unwrap_or_default();
                    self.match_idx = 0;
                }
                _ => {}
            }
        }
    }

    fn close(&mut self) {
        self.outer.close();
        self.table.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::super::stages::{VecScan, collect_all};
    use super::*;

    fn kv(k: i64, v: i64) -> i64 {
        k * 1000 + v
    }
    fn key(r: i64) -> i64 {
        r / 1000
    }
    fn val(r: i64) -> i64 {
        r % 1000
    }

    // ZERO: both sides empty
    #[test]
    fn both_empty() {
        let outer = Box::new(VecScan::new(vec![]));
        let inner = Box::new(VecScan::new(vec![]));
        let mut j = HashJoinStage::new(outer, inner, key, key, val, val);
        j.open(false);
        assert_eq!(collect_all(&mut j), vec![]);
    }

    // MANY
    #[test]
    fn basic_join() {
        let outer = Box::new(VecScan::new(vec![kv(1, 10), kv(2, 20), kv(3, 30)]));
        let inner = Box::new(VecScan::new(vec![kv(2, 5), kv(3, 6), kv(4, 7)]));
        let mut j = HashJoinStage::new(outer, inner, key, key, val, val);
        j.open(false);
        let mut r = collect_all(&mut j);
        r.sort();
        assert_eq!(r, vec![kv(20, 5), kv(30, 6)]);
    }

    // EDGE: no matching keys
    #[test]
    fn no_matches() {
        let outer = Box::new(VecScan::new(vec![kv(1, 10)]));
        let inner = Box::new(VecScan::new(vec![kv(2, 20)]));
        let mut j = HashJoinStage::new(outer, inner, key, key, val, val);
        j.open(false);
        assert_eq!(collect_all(&mut j), vec![]);
    }

    // EDGE: multiple inner matches per key
    #[test]
    fn multiple_matches() {
        let outer = Box::new(VecScan::new(vec![kv(1, 10)]));
        let inner = Box::new(VecScan::new(vec![kv(1, 1), kv(1, 2)]));
        let mut j = HashJoinStage::new(outer, inner, key, key, val, val);
        j.open(false);
        let mut r = collect_all(&mut j);
        r.sort();
        assert_eq!(r, vec![kv(10, 1), kv(10, 2)]);
    }

    // EDGE: empty inner side
    #[test]
    fn empty_inner() {
        let outer = Box::new(VecScan::new(vec![kv(1, 10)]));
        let inner = Box::new(VecScan::new(vec![]));
        let mut j = HashJoinStage::new(outer, inner, key, key, val, val);
        j.open(false);
        assert_eq!(collect_all(&mut j), vec![]);
    }
}
