// Pull-Based Iterator Stages — SBE Execution Model
//
// MongoDB's SBE engine is a tree of stages connected by slots.
// Each stage implements: open(reOpen) -> getNext() -> close()
// Parent pulls from children (Volcano model, row-at-a-time).
//
// This file contains the Stage trait and VecScan mock (shared infrastructure).
// Each stage is a separate file scoped for 20 min implementation + 10 min tests.
// In a real interview you'd implement ONE stage, not all of them.

/// Execution state returned by `get_next()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanState {
    Advanced,
    Eof,
}

/// Pull-based stage interface. Mirrors `sbe::PlanStage`.
///
/// Lifecycle: `open()` -> `get_next()` in loop -> `close()`.
/// `open(re_open=true)` resets state without full reconstruction.
pub trait Stage {
    /// Acquire resources. If `re_open` is true, reset without full reconstruction.
    fn open(&mut self, re_open: bool);

    /// Produce the next row. Returns (`Advanced`, `Some(value)`) or (`Eof`, `None`).
    fn get_next(&mut self) -> (PlanState, Option<i64>);

    /// Release resources.
    fn close(&mut self);
}

/// Drain all output from a stage into a `Vec`. O(n). Test helper.
pub fn collect_all(stage: &mut dyn Stage) -> Vec<i64> {
    let mut results = Vec::new();
    while let (PlanState::Advanced, Some(val)) = stage.get_next() {
        results.push(val);
    }
    results
}

/// Mock data source producing rows from a `Vec`. O(1) per `get_next()`.
/// Equivalent to SBE's `VirtualScanStage`.
pub struct VecScan {
    data: Vec<i64>,
    idx: usize,
}

impl VecScan {
    /// O(1).
    #[must_use]
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
