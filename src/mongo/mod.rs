// MongoDB-specific patterns: pull-based SBE execution stages.
//
// Each stage is a separate file, scoped for one 45-min interview:
//   20 min implementation + 10 min tests + 15 min discussion.
//
// Shared infrastructure (Stage trait, VecScan, collect_all) is in stages.rs.
// In a real interview you implement ONE of these, not all.

/// Stage trait, `PlanState` enum, `VecScan` mock, `collect_all` helper.
pub mod stages;

/// `FilterStage` -- streaming predicate filter. Simplest stage to implement.
pub mod filter_stage;

/// `LimitSkipStage` -- skip N then return M. Tests `reOpen` semantics.
pub mod limit_skip_stage;

/// `HashAggStage` -- blocking GROUP BY with SUM. Tests hash table + accumulator.
pub mod hash_agg_stage;

/// `HashJoinStage` -- build/probe equi-join. Tests hash table + multi-match buffering.
pub mod hash_join_stage;
