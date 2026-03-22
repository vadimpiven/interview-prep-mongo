// Pull-Based Iterator Stages — SBE Execution Model
//
// MongoDB's SBE engine is a tree of stages connected by slots.
// Each stage implements: open(reOpen) → getNext() → close()
// Parent pulls from children (Volcano model, row-at-a-time).
//
// This module implements simplified versions of real SBE stages:
//   VecScan     — mock data source (like VirtualScanStage)
//   Filter      — pass through matching rows (like FilterStage)
//   LimitSkip   — skip N then return M (like LimitSkipStage)
//   HashAgg     — GROUP BY with SUM accumulator (like HashAggStage)
//   HashJoin    — build/probe hash join (like HashJoinStage)
//
// In real SBE, stages communicate via named slots (SlotId → SlotAccessor).
// Here we simplify to i64 values for clarity.

use std::collections::HashMap;

/// Execution state returned by get_next().
/// MongoDB: enum class PlanState { ADVANCED, IS_EOF };
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanState {
    Advanced,
    Eof,
}

/// Simplified stage interface.
/// Real SBE also has: prepare(), getAccessor(), saveState(), restoreState().
pub trait Stage {
    fn open(&mut self, re_open: bool);
    fn get_next(&mut self) -> (PlanState, Option<i64>);
    fn close(&mut self);
}

/// Collect all output from a stage into a Vec. Useful in tests.
pub fn collect_all(stage: &mut dyn Stage) -> Vec<i64> {
    let mut results = Vec::new();
    loop {
        match stage.get_next() {
            (PlanState::Advanced, Some(val)) => results.push(val),
            _ => break,
        }
    }
    results
}

// ---------------------------------------------------------------------------
// VecScan — Mock data source
// ---------------------------------------------------------------------------

/// Produces rows from a Vec. Equivalent to SBE's VirtualScanStage.
pub struct VecScan {
    data: Vec<i64>,
    idx: usize,
}

impl VecScan {
    pub fn new(data: Vec<i64>) -> Self {
        Self { data, idx: 0 }
    }
}

impl Stage for VecScan {
    fn open(&mut self, _re_open: bool) {
        self.idx = 0;
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        if self.idx < self.data.len() {
            let val = self.data[self.idx];
            self.idx += 1;
            (PlanState::Advanced, Some(val))
        } else {
            (PlanState::Eof, None)
        }
    }

    fn close(&mut self) {}
}

// ---------------------------------------------------------------------------
// FilterStage — Pass through rows matching a predicate
// ---------------------------------------------------------------------------

/// Pulls from child, returns only rows where predicate returns true.
/// Streaming: no buffering, O(1) memory overhead.
pub struct FilterStage {
    child: Box<dyn Stage>,
    predicate: Box<dyn Fn(i64) -> bool>,
    is_eof: bool,
}

impl FilterStage {
    pub fn new(child: Box<dyn Stage>, predicate: impl Fn(i64) -> bool + 'static) -> Self {
        Self {
            child,
            predicate: Box::new(predicate),
            is_eof: false,
        }
    }
}

impl Stage for FilterStage {
    fn open(&mut self, re_open: bool) {
        self.child.open(re_open);
        self.is_eof = false;
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        while !self.is_eof {
            match self.child.get_next() {
                (PlanState::Eof, _) => {
                    self.is_eof = true;
                    return (PlanState::Eof, None);
                }
                (PlanState::Advanced, Some(val)) if (self.predicate)(val) => {
                    return (PlanState::Advanced, Some(val));
                }
                _ => continue, // doesn't match — pull next
            }
        }
        (PlanState::Eof, None)
    }

    fn close(&mut self) {
        self.child.close();
    }
}

// ---------------------------------------------------------------------------
// LimitSkipStage — Skip N rows, then return at most M rows
// ---------------------------------------------------------------------------

/// Blocking in open() for skip phase, streaming for limit phase.
/// Real SBE: src/mongo/db/exec/sbe/stages/limit_skip.cpp
pub struct LimitSkipStage {
    child: Box<dyn Stage>,
    skip: usize,
    limit: usize,
    current: usize,
    is_eof: bool,
}

impl LimitSkipStage {
    pub fn new(child: Box<dyn Stage>, skip: usize, limit: usize) -> Self {
        Self {
            child,
            skip,
            limit,
            current: 0,
            is_eof: false,
        }
    }
}

impl Stage for LimitSkipStage {
    fn open(&mut self, re_open: bool) {
        self.child.open(re_open);
        self.current = 0;
        self.is_eof = false;
        // Drain skip rows from child
        for _ in 0..self.skip {
            if let (PlanState::Eof, _) = self.child.get_next() {
                self.is_eof = true;
                return;
            }
        }
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        if self.is_eof || self.current >= self.limit {
            return (PlanState::Eof, None);
        }
        self.current += 1;
        self.child.get_next()
    }

    fn close(&mut self) {
        self.child.close();
    }
}

// ---------------------------------------------------------------------------
// HashAggStage — GROUP BY with SUM accumulator
// ---------------------------------------------------------------------------

/// Blocking: consumes all input in open(), builds hash table of groups.
/// getNext() iterates over completed groups.
///
/// Real SBE: src/mongo/db/exec/sbe/stages/hash_agg.cpp
/// Real version also supports: spilling to disk, multiple accumulators,
/// collation-aware grouping, memory tracking.
///
/// Encoding: we pack (key, value) into a single i64 as key=row/1000, val=row%1000.
/// In real SBE, key and value are separate slots.
pub struct HashAggStage {
    child: Box<dyn Stage>,
    key_fn: fn(i64) -> i64,
    val_fn: fn(i64) -> i64,
    groups: Vec<(i64, i64)>, // (key, sum)
    idx: usize,
}

impl HashAggStage {
    pub fn new(child: Box<dyn Stage>, key_fn: fn(i64) -> i64, val_fn: fn(i64) -> i64) -> Self {
        Self {
            child,
            key_fn,
            val_fn,
            groups: Vec::new(),
            idx: 0,
        }
    }
}

impl Stage for HashAggStage {
    fn open(&mut self, re_open: bool) {
        self.child.open(re_open);
        let mut map: HashMap<i64, i64> = HashMap::new();

        // Blocking: consume all input, accumulate into hash table
        loop {
            match self.child.get_next() {
                (PlanState::Advanced, Some(row)) => {
                    let key = (self.key_fn)(row);
                    let val = (self.val_fn)(row);
                    *map.entry(key).or_insert(0) += val;
                }
                _ => break,
            }
        }
        self.child.close();

        // Materialize groups in sorted order for deterministic output
        self.groups = map.into_iter().collect();
        self.groups.sort_by_key(|(k, _)| *k);
        self.idx = 0;
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        if self.idx >= self.groups.len() {
            return (PlanState::Eof, None);
        }
        let (_, sum) = self.groups[self.idx];
        self.idx += 1;
        (PlanState::Advanced, Some(sum))
    }

    fn close(&mut self) {
        self.groups.clear();
    }
}

// ---------------------------------------------------------------------------
// HashJoinStage — Build/probe equi-join
// ---------------------------------------------------------------------------

/// Build phase (in open): drain inner child into hash table keyed by join key.
/// Probe phase (in get_next): for each outer row, probe hash table.
///
/// Real SBE: src/mongo/db/exec/sbe/stages/hash_join.cpp
/// Real version also has: HybridHashJoinStage with spilling + bloom filter.
///
/// Encoding: join key = row / 1000, payload = row % 1000.
/// Output: outer_payload * 1000 + inner_payload (to verify both sides matched).
pub struct HashJoinStage {
    outer: Box<dyn Stage>,
    inner: Box<dyn Stage>,
    outer_key_fn: fn(i64) -> i64,
    inner_key_fn: fn(i64) -> i64,
    outer_val_fn: fn(i64) -> i64,
    inner_val_fn: fn(i64) -> i64,
    table: HashMap<i64, Vec<i64>>, // join_key → [inner_payload, ...]
    // Buffered matches for current outer row
    match_buffer: Vec<i64>,
    match_idx: usize,
    current_outer_val: i64,
    is_eof: bool,
}

impl HashJoinStage {
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
            match_buffer: Vec::new(),
            match_idx: 0,
            current_outer_val: 0,
            is_eof: false,
        }
    }
}

impl Stage for HashJoinStage {
    fn open(&mut self, re_open: bool) {
        // Build phase: materialize inner side into hash table
        self.inner.open(re_open);
        self.table.clear();
        loop {
            match self.inner.get_next() {
                (PlanState::Advanced, Some(row)) => {
                    let key = (self.inner_key_fn)(row);
                    let val = (self.inner_val_fn)(row);
                    self.table.entry(key).or_default().push(val);
                }
                _ => break,
            }
        }
        self.inner.close();

        // Prepare probe phase
        self.outer.open(re_open);
        self.match_buffer.clear();
        self.match_idx = 0;
        self.is_eof = false;
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        loop {
            // Return buffered matches from current outer row
            if self.match_idx < self.match_buffer.len() {
                let inner_val = self.match_buffer[self.match_idx];
                self.match_idx += 1;
                let output = self.current_outer_val * 1000 + inner_val;
                return (PlanState::Advanced, Some(output));
            }

            if self.is_eof {
                return (PlanState::Eof, None);
            }

            // Pull next outer row and probe
            match self.outer.get_next() {
                (PlanState::Eof, _) => {
                    self.is_eof = true;
                    return (PlanState::Eof, None);
                }
                (PlanState::Advanced, Some(row)) => {
                    let key = (self.outer_key_fn)(row);
                    self.current_outer_val = (self.outer_val_fn)(row);
                    self.match_buffer = self.table.get(&key).cloned().unwrap_or_default();
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
    use super::*;

    // -- Filter tests --

    #[test]
    fn filter_basic() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3, 4, 5]));
        let mut filter = FilterStage::new(scan, |v| v > 3);
        filter.open(false);
        assert_eq!(collect_all(&mut filter), vec![4, 5]);
    }

    #[test]
    fn filter_no_match() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3]));
        let mut filter = FilterStage::new(scan, |v| v > 100);
        filter.open(false);
        assert_eq!(collect_all(&mut filter), vec![]);
    }

    #[test]
    fn filter_empty_input() {
        let scan = Box::new(VecScan::new(vec![]));
        let mut filter = FilterStage::new(scan, |_| true);
        filter.open(false);
        assert_eq!(collect_all(&mut filter), vec![]);
    }

    #[test]
    fn filter_all_match() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3]));
        let mut filter = FilterStage::new(scan, |_| true);
        filter.open(false);
        assert_eq!(collect_all(&mut filter), vec![1, 2, 3]);
    }

    // -- LimitSkip tests --

    #[test]
    fn limit_skip_basic() {
        let scan = Box::new(VecScan::new(vec![10, 20, 30, 40, 50]));
        let mut ls = LimitSkipStage::new(scan, 1, 2); // skip 1, limit 2
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![20, 30]);
    }

    #[test]
    fn limit_exceeds_input() {
        let scan = Box::new(VecScan::new(vec![10, 20]));
        let mut ls = LimitSkipStage::new(scan, 0, 100);
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![10, 20]);
    }

    #[test]
    fn skip_all() {
        let scan = Box::new(VecScan::new(vec![10, 20, 30]));
        let mut ls = LimitSkipStage::new(scan, 10, 100);
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![]);
    }

    #[test]
    fn reopen_resets_state() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3]));
        let mut ls = LimitSkipStage::new(scan, 0, 2);
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![1, 2]);
        ls.open(true); // re-open should reset
        assert_eq!(collect_all(&mut ls), vec![1, 2]);
    }

    // -- HashAgg tests --

    #[test]
    fn hash_agg_group_by_sum() {
        // Encoding: key = row / 10, value = row % 10
        // Rows: (key=1,val=5), (key=1,val=3), (key=2,val=7), (key=2,val=1)
        let scan = Box::new(VecScan::new(vec![15, 13, 27, 21]));
        let mut agg = HashAggStage::new(scan, |r| r / 10, |r| r % 10);
        agg.open(false);
        let results = collect_all(&mut agg);
        // key=1: 5+3=8, key=2: 7+1=8
        assert_eq!(results, vec![8, 8]);
    }

    #[test]
    fn hash_agg_empty_input() {
        let scan = Box::new(VecScan::new(vec![]));
        let mut agg = HashAggStage::new(scan, |r| r, |r| r);
        agg.open(false);
        assert_eq!(collect_all(&mut agg), vec![]);
    }

    #[test]
    fn hash_agg_single_group() {
        let scan = Box::new(VecScan::new(vec![11, 12, 13]));
        let mut agg = HashAggStage::new(scan, |r| r / 10, |r| r % 10);
        agg.open(false);
        assert_eq!(collect_all(&mut agg), vec![6]); // 1+2+3
    }

    // -- HashJoin tests --

    #[test]
    fn hash_join_basic() {
        // Outer: key=1 val=10, key=2 val=20, key=3 val=30
        // Inner: key=2 val=5, key=3 val=6, key=4 val=7
        // Expected matches: (key=2: 20*1000+5=20005), (key=3: 30*1000+6=30006)
        let outer = Box::new(VecScan::new(vec![1010, 2020, 3030]));
        let inner = Box::new(VecScan::new(vec![2005, 3006, 4007]));
        let mut join = HashJoinStage::new(
            outer,
            inner,
            |r| r / 1000, // outer key
            |r| r / 1000, // inner key
            |r| r % 1000, // outer payload
            |r| r % 1000, // inner payload
        );
        join.open(false);
        let mut results = collect_all(&mut join);
        results.sort();
        assert_eq!(results, vec![20005, 30006]);
    }

    #[test]
    fn hash_join_no_matches() {
        let outer = Box::new(VecScan::new(vec![1010]));
        let inner = Box::new(VecScan::new(vec![2020]));
        let mut join = HashJoinStage::new(
            outer,
            inner,
            |r| r / 1000,
            |r| r / 1000,
            |r| r % 1000,
            |r| r % 1000,
        );
        join.open(false);
        assert_eq!(collect_all(&mut join), vec![]);
    }

    #[test]
    fn hash_join_multiple_matches() {
        // Inner has two rows with key=1
        let outer = Box::new(VecScan::new(vec![1010]));
        let inner = Box::new(VecScan::new(vec![1001, 1002]));
        let mut join = HashJoinStage::new(
            outer,
            inner,
            |r| r / 1000,
            |r| r / 1000,
            |r| r % 1000,
            |r| r % 1000,
        );
        join.open(false);
        let mut results = collect_all(&mut join);
        results.sort();
        // outer_val=10, inner_vals=1,2 → 10*1000+1=10001, 10*1000+2=10002
        assert_eq!(results, vec![10001, 10002]);
    }

    #[test]
    fn hash_join_empty_inner() {
        let outer = Box::new(VecScan::new(vec![1010, 2020]));
        let inner = Box::new(VecScan::new(vec![]));
        let mut join = HashJoinStage::new(
            outer,
            inner,
            |r| r / 1000,
            |r| r / 1000,
            |r| r % 1000,
            |r| r % 1000,
        );
        join.open(false);
        assert_eq!(collect_all(&mut join), vec![]);
    }

    #[test]
    fn hash_join_empty_outer() {
        let outer = Box::new(VecScan::new(vec![]));
        let inner = Box::new(VecScan::new(vec![1001, 2002]));
        let mut join = HashJoinStage::new(
            outer,
            inner,
            |r| r / 1000,
            |r| r / 1000,
            |r| r % 1000,
            |r| r % 1000,
        );
        join.open(false);
        assert_eq!(collect_all(&mut join), vec![]);
    }

    // -- Composed pipeline test --

    #[test]
    fn filter_then_limit() {
        // Pipeline: Scan [1..10] → Filter(>5) → LimitSkip(skip=1, limit=2)
        // Filter produces: 6, 7, 8, 9, 10
        // Skip 1: 7, 8, 9, 10
        // Limit 2: 7, 8
        let scan = Box::new(VecScan::new((1..=10).collect()));
        let filter = Box::new(FilterStage::new(scan, |v| v > 5));
        let mut ls = LimitSkipStage::new(filter, 1, 2);
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![7, 8]);
    }
}
