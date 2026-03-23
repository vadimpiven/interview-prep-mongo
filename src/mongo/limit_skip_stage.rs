// LimitSkipStage — skip N rows, then return at most M
//
// Scoped for 20 min: implement struct + Stage trait.
// open() drains skip rows (blocking for skip phase), getNext() counts limit.
// Real SBE: src/mongo/db/exec/sbe/stages/limit_skip.cpp

use super::stages::{PlanState, Stage};

pub struct LimitSkipStage {
    child: Box<dyn Stage>,
    skip: usize,
    limit: usize,
    current: usize,
    is_eof: bool,
}

impl LimitSkipStage {
    /// O(1).
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::super::stages::{VecScan, collect_all};
    use super::*;

    // ZERO
    #[test]
    fn empty_input() {
        let scan = Box::new(VecScan::new(vec![]));
        let mut ls = LimitSkipStage::new(scan, 0, 10);
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![]);
    }

    // MANY
    #[test]
    fn basic() {
        let scan = Box::new(VecScan::new(vec![10, 20, 30, 40, 50]));
        let mut ls = LimitSkipStage::new(scan, 1, 2);
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
    fn reopen_resets() {
        let scan = Box::new(VecScan::new(vec![1, 2, 3]));
        let mut ls = LimitSkipStage::new(scan, 0, 2);
        ls.open(false);
        assert_eq!(collect_all(&mut ls), vec![1, 2]);
        ls.open(true);
        assert_eq!(collect_all(&mut ls), vec![1, 2]);
    }
}
