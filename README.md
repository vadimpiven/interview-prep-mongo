# MongoDB Query Execution — Interview Preparation

## Position

[Senior Software Engineer, Query Execution](https://www.mongodb.com/careers/jobs/7484657) — Dublin / Remote Ireland

| Field              | Details                                              |
| ------------------ | ---------------------------------------------------- |
| **Team**           | Query Execution, Product & Technology                |
| **Level**          | Senior (IC), 5+ years systems programming            |
| **Languages**      | C++ (codebase), C, Rust, or similar compiled         |
| **Focus areas**    | Query performance, language enhancements, diagnostics |
| **Preferred**      | Database / query engine experience; MS/PhD in DB     |

## Interview

| Field           | Details                                           |
| --------------- | ------------------------------------------------- |
| **Type**        | Technical Screen — coding + discussion            |
| **Duration**    | 45 min coding, 15 min questions                   |
| **Platform**    | [CoderPad](https://app.coderpad.io/sandbox)       |
| **Expectation** | Compilable code + passing tests, not pseudocode   |
| **Repo**        | [mongodb/mongo](https://github.com/mongodb/mongo) |

### Timing

```
1. Clarify the problem                          5 min
2. Write the struct / function                  20 min
3. Write tests                                  10 min
4. Build + fix compile errors                    5 min
5. Run tests + fix failures                      5 min
```

## Interviewers

| Role             | Name                                                              | Team                       | Location |
| ---------------- | ----------------------------------------------------------------- | -------------------------- | -------- |
| **Leads coding** | [Catalin Sumanaru](https://www.linkedin.com/in/catalin-sumanaru/) | Query Execution (SE3)      | Dublin   |
| **Assists**      | [Lyublena Antova](https://www.linkedin.com/in/antova/)            | Query Optimization (Staff) | Bulgaria |

## Team

| Role            | Name                                                                   | Location |
| --------------- | ---------------------------------------------------------------------- | -------- |
| Manager         | [Niels Lohmann](https://www.linkedin.com/in/nielslohmann/)             | Germany  |
| Senior Engineer | [Nicola Cabiddu](https://www.linkedin.com/in/nicola-cabiddu-78a15351/) | Dublin   |
| Engineer (SE3)  | [Catalin Sumanaru](https://www.linkedin.com/in/catalin-sumanaru/)      | Dublin   |
| Engineer        | _(unknown)_                                                            | Dublin   |

Niels Lohmann posted two open roles: 1 senior + 1 mid. The team is part of
the Query Execution (QE) group within MongoDB's ~80-person Query organization.
QE owns the execution engines (SBE and Classic), stage builders, yielding,
router execution, and write execution. The cost-based optimizer (`compiler/`)
is owned by a separate Query Optimization (QO) team.

## Team Focus — Last 3 Months

### Catalin Sumanaru (interviewer)

**SBE `getField` optimization** — performance-critical hot path.
Hybrid approach: detect field name length via null-byte position checks
instead of `strlen()`, byte-by-byte comparison for short names, `memcmp` for long.

- [SERVER-121006 #49102](https://github.com/mongodb/mongo/pull/49102) — Hybrid `sbe::getField()` for short & long field names
- [SERVER-120255 #47981](https://github.com/mongodb/mongo/pull/47981) — Add benchmarks, optimize `bson::getField()`
- Codebase: [`sbe/values/bson.h` getField](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/values/bson.h), [`sbe_get_field_bm.cpp`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/sbe_get_field_bm.cpp)

**Query Settings Backfill** — auto-populate persistent query settings from observed patterns.

- [SERVER-104937 #36675](https://github.com/mongodb/mongo/pull/36675) — Implement PQS backfilling algorithm
- [SERVER-105476 #38288](https://github.com/mongodb/mongo/pull/38288) — Cluster + replica set BackfillCoordinator
- [SERVER-107106 #38578](https://github.com/mongodb/mongo/pull/38578) — Backfill server status metrics
- Codebase: [`query_settings/`](https://github.com/mongodb/mongo/tree/master/src/mongo/db/query/query_settings)

**Invariant → tassert migration** — systematic audit replacing `invariant()` (crash) with `tassert()` (log, don't crash).

- [SERVER-113213 #44115](https://github.com/mongodb/mongo/pull/44115) — Audit invariants: executors (classic + SBE)
- [SERVER-113212 #43873](https://github.com/mongodb/mongo/pull/43873) — Audit invariants: plan ranking and trial execution
- [SERVER-113209 #43822](https://github.com/mongodb/mongo/pull/43822) — Audit invariants: canonical query and command parsing

**DocumentSource QO/QE splitting** — separating pipeline stages into planning (QO) and execution (QE).

- [SERVER-109801 #40904](https://github.com/mongodb/mongo/pull/40904) — Split DocumentSourceInternalShredDocuments
- [SERVER-108170 #39689](https://github.com/mongodb/mongo/pull/39689) — Split DocumentSourceQueue
- Codebase: [`db/pipeline/`](https://github.com/mongodb/mongo/tree/master/src/mongo/db/pipeline)

### Nicola Cabiddu (senior)

**Change stream v2 testing framework** — correctness verification comparing v1 vs v2.

- [SERVER-114585 #45612](https://github.com/mongodb/mongo/pull/45612) — Verifier (FetchOneAndResume, PrefixRead, V1 vs V2)
- [SERVER-113291 #48910](https://github.com/mongodb/mongo/pull/48910) — Parallel reader and writer
- [SERVER-119267 #49163](https://github.com/mongodb/mongo/pull/49163) — Background mutator in its own thread

**MongoR** — query replay tool for performance testing.

- [SERVER-104470 #35525](https://github.com/mongodb/mongo/pull/35525) — Initial commit mongoR
- [SERVER-106047 #37132](https://github.com/mongodb/mongo/pull/37132) — Session simulation

### Niels Lohmann (manager)

Pipeline refactoring (move APIs to `exec::agg` namespace), code quality
(Coverity fixes, null checks, tasserts), modularity markers for team ownership.

### Relevant Codebase Files

| Topic                  | MongoDB Source                                                                                                           |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| SBE stage interface    | [`exec/sbe/stages/stages.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/stages/stages.h)         |
| SBE value types        | [`exec/sbe/values/value.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/values/value.h)           |
| Hash join stage        | [`exec/sbe/stages/hash_join.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/stages/hash_join.h)   |
| Hash aggregation       | [`exec/sbe/stages/hash_agg.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/stages/hash_agg.h)     |
| Sort stage             | [`exec/sbe/stages/sort.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/stages/sort.h)             |
| LimitSkip stage        | [`exec/sbe/stages/limit_skip.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/stages/limit_skip.h) |
| Bloom filter           | [`exec/sbe/util/bloom_filter.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/util/bloom_filter.h) |
| Sorter (external sort) | [`sorter/`](https://github.com/mongodb/mongo/tree/master/src/mongo/db/sorter)                                            |
| LRU plan cache         | [`query/lru_key_value.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/query/lru_key_value.h)               |
| MatchExpression tree   | [`matcher/expression.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/matcher/expression.h)                 |
| Tree walker            | [`query/tree_walker.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/query/tree_walker.h)                   |
| BSON field lookup      | [`exec/sbe/values/bson.h`](https://github.com/mongodb/mongo/blob/master/src/mongo/db/exec/sbe/values/bson.h)             |
