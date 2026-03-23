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

/// Collect all output from a stage into a Vec, then close it.
/// O(N) where N = total rows produced by the stage.
pub fn collect_all(stage: &mut dyn Stage) -> Vec<i64> {
    let mut results = Vec::new();
    loop {
        match stage.get_next() {
            (PlanState::Advanced, Some(val)) => results.push(val),
            (PlanState::Advanced, None) => {
                unreachable!("stage returned Advanced without a value")
            }
            (PlanState::Eof, _) => break,
        }
    }
    stage.close();
    results
}

// ---------------------------------------------------------------------------
// VecScan — Mock data source
// ---------------------------------------------------------------------------

/// Produces rows from a Vec. Equivalent to SBE's VirtualScanStage.
/// open O(1), get_next O(1), close O(1). Space O(n) for stored data.
pub struct VecScan {
    data: Vec<i64>,
    cursor: usize,
}

impl VecScan {
    pub fn new(data: Vec<i64>) -> Self {
        Self { data, cursor: 0 }
    }
}

impl Stage for VecScan {
    fn open(&mut self, _re_open: bool) {
        self.cursor = 0;
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        if self.cursor < self.data.len() {
            let val = self.data[self.cursor];
            self.cursor += 1;
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
/// open O(1), get_next O(k) where k = rows skipped before next match, close O(1).
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
/// open O(skip), get_next O(1), close O(1).
pub struct LimitSkipStage {
    child: Box<dyn Stage>,
    skip: usize,
    limit: usize,
    emitted: usize,
    is_eof: bool,
}

impl LimitSkipStage {
    pub fn new(child: Box<dyn Stage>, skip: usize, limit: usize) -> Self {
        Self {
            child,
            skip,
            limit,
            emitted: 0,
            is_eof: false,
        }
    }
}

impl Stage for LimitSkipStage {
    fn open(&mut self, re_open: bool) {
        self.child.open(re_open);
        self.emitted = 0;
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
        if self.is_eof || self.emitted >= self.limit {
            return (PlanState::Eof, None);
        }
        let result = self.child.get_next();
        if matches!(result, (PlanState::Advanced, _)) {
            self.emitted += 1;
        }
        result
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
/// Output: key * 1000 + sum (so consumers can identify which group each sum belongs to).
/// In real SBE, key and value are separate slots.
///
/// open O(N) blocking — consumes all N input rows into hash table.
/// get_next O(1) — iterates over materialized groups.
/// close O(G) — clears G groups. Space O(G) where G = number of distinct groups.
pub struct HashAggStage {
    child: Box<dyn Stage>,
    key_fn: fn(i64) -> i64,
    val_fn: fn(i64) -> i64,
    groups: Vec<(i64, i64)>, // (key, sum)
    cursor: usize,
}

impl HashAggStage {
    pub fn new(child: Box<dyn Stage>, key_fn: fn(i64) -> i64, val_fn: fn(i64) -> i64) -> Self {
        Self {
            child,
            key_fn,
            val_fn,
            groups: Vec::new(),
            cursor: 0,
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
        // Materialize groups in sorted order for deterministic output
        self.groups = map.into_iter().collect();
        self.groups.sort_by_key(|(k, _)| *k);
        self.cursor = 0;
    }

    fn get_next(&mut self) -> (PlanState, Option<i64>) {
        if self.cursor >= self.groups.len() {
            return (PlanState::Eof, None);
        }
        let (key, sum) = self.groups[self.cursor];
        self.cursor += 1;
        (PlanState::Advanced, Some(key * 1000 + sum))
    }

    fn close(&mut self) {
        self.child.close();
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
///
/// open O(I) — builds hash table from I inner rows.
/// get_next O(1) amortized — probes hash table per outer row.
/// close O(I) — clears build table. Space O(I) for the build side.
pub struct HashJoinStage {
    outer: Box<dyn Stage>,
    inner: Box<dyn Stage>,
    outer_key_fn: fn(i64) -> i64,
    inner_key_fn: fn(i64) -> i64,
    outer_val_fn: fn(i64) -> i64,
    inner_val_fn: fn(i64) -> i64,
    build_table: HashMap<i64, Vec<i64>>, // join_key → [inner_payload, ...]
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
            build_table: HashMap::new(),
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
        self.build_table.clear();
        loop {
            match self.inner.get_next() {
                (PlanState::Advanced, Some(row)) => {
                    let key = (self.inner_key_fn)(row);
                    let val = (self.inner_val_fn)(row);
                    self.build_table.entry(key).or_default().push(val);
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
                    self.match_buffer = self.build_table.get(&key).cloned().unwrap_or_default();
                    self.match_idx = 0;
                }
                _ => unreachable!("outer stage returned Advanced without a value"),
            }
        }
    }

    fn close(&mut self) {
        self.outer.close();
        self.build_table.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_basic() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3, 4, 5]));
        let mut filter = FilterStage::new(scan, |v| v > 3);
        filter.open(false);
        assert_eq!(collect_all(&mut filter), vec![4, 5]);
    }

    #[test]
    fn limit_skip_basic() {
        let scan = Box::new(VecScan::new(vec![10, 20, 30, 40, 50]));
        let mut ls = LimitSkipStage::new(scan, 1, 2); // skip 1, limit 2
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![20, 30]);
    }

    #[test]
    fn hash_agg_group_by_sum() {
        // Encoding: key = row / 10, value = row % 10
        // Rows: (key=1,val=5), (key=1,val=3), (key=2,val=7), (key=2,val=1)
        // Output: key*1000+sum → 1008, 2008
        let scan = Box::new(VecScan::new(vec![15, 13, 27, 21]));
        let mut agg = HashAggStage::new(scan, |r| r / 10, |r| r % 10);
        agg.open(false);
        let results = collect_all(&mut agg);
        assert_eq!(results, vec![1008, 2008]);
    }

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
