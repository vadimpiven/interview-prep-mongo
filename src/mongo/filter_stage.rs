// FilterStage — pass through rows matching a predicate
//
// Scoped for 20 min: implement FilterStage struct + Stage trait.
// Streaming: no buffering, O(1) memory. Pull from child until match or EOF.
// Real SBE: src/mongo/db/exec/sbe/stages/filter.h

use super::stages::{PlanState, Stage};

pub struct FilterStage {
    child: Box<dyn Stage>,
    predicate: Box<dyn Fn(i64) -> bool>,
    is_eof: bool,
}

impl FilterStage {
    /// O(1).
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
                _ => {}
            }
        }
        (PlanState::Eof, None)
    }

    fn close(&mut self) {
        self.child.close();
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
        let mut f = FilterStage::new(scan, |_| true);
        f.open(false);
        assert_eq!(collect_all(&mut f), vec![]);
    }

    // ONE
    #[test]
    fn single_match() {
        let scan = Box::new(VecScan::new(vec![5]));
        let mut f = FilterStage::new(scan, |v| v > 3);
        f.open(false);
        assert_eq!(collect_all(&mut f), vec![5]);
    }

    // MANY
    #[test]
    fn basic() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3, 4, 5]));
        let mut f = FilterStage::new(scan, |v| v > 3);
        f.open(false);
        assert_eq!(collect_all(&mut f), vec![4, 5]);
    }

    // EDGE: predicate matches nothing
    #[test]
    fn no_match() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3]));
        let mut f = FilterStage::new(scan, |v| v > 100);
        f.open(false);
        assert_eq!(collect_all(&mut f), vec![]);
    }

    // EDGE: reopen resets
    #[test]
    fn reopen_resets() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3]));
        let mut f = FilterStage::new(scan, |v| v > 1);
        f.open(false);
        assert_eq!(collect_all(&mut f), vec![2, 3]);
        f.open(true);
        assert_eq!(collect_all(&mut f), vec![2, 3]);
    }
}
