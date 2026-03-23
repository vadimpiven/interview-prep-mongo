// HashAggStage — GROUP BY with SUM accumulator
//
// Scoped for 20 min: implement struct + Stage trait.
// Blocking: consumes all input in open(), builds hash table.
// getNext() iterates over completed groups.
// Real SBE: src/mongo/db/exec/sbe/stages/hash_agg.cpp
//
// Encoding: key = row / 1000, value = row % 1000.
// In real SBE, key and value are separate slots.

use super::stages::{PlanState, Stage};
use std::collections::HashMap;

pub struct HashAggStage {
    child: Box<dyn Stage>,
    key_fn: fn(i64) -> i64,
    val_fn: fn(i64) -> i64,
    groups: Vec<(i64, i64)>,
    idx: usize,
}

impl HashAggStage {
    /// O(1).
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
        while let (PlanState::Advanced, Some(row)) = self.child.get_next() {
            let key = (self.key_fn)(row);
            let val = (self.val_fn)(row);
            *map.entry(key).or_insert(0) += val;
        }
        self.child.close();
        self.groups = map.into_iter().collect();
        self.groups.sort_by_key(|(k, _)| *k); // deterministic output
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

#[cfg(test)]
mod tests {
    use super::super::stages::{VecScan, collect_all};
    use super::*;

    // ZERO
    #[test]
    fn empty_input() {
        let scan = Box::new(VecScan::new(vec![]));
        let mut agg = HashAggStage::new(scan, |r| r, |r| r);
        agg.open(false);
        assert_eq!(collect_all(&mut agg), vec![]);
    }

    // ONE
    #[test]
    fn single_row() {
        let scan = Box::new(VecScan::new(vec![15])); // key=1, val=5
        let mut agg = HashAggStage::new(scan, |r| r / 10, |r| r % 10);
        agg.open(false);
        assert_eq!(collect_all(&mut agg), vec![5]);
    }

    // MANY
    #[test]
    fn group_by_sum() {
        // key=row/10, val=row%10. Rows: (1,5),(1,3),(2,7),(2,1)
        let scan = Box::new(VecScan::new(vec![15, 13, 27, 21]));
        let mut agg = HashAggStage::new(scan, |r| r / 10, |r| r % 10);
        agg.open(false);
        assert_eq!(collect_all(&mut agg), vec![8, 8]); // key1: 5+3, key2: 7+1
    }

    // EDGE: many rows, one group
    #[test]
    fn single_group() {
        let scan = Box::new(VecScan::new(vec![11, 12, 13]));
        let mut agg = HashAggStage::new(scan, |r| r / 10, |r| r % 10);
        agg.open(false);
        assert_eq!(collect_all(&mut agg), vec![6]); // 1+2+3
    }
}
