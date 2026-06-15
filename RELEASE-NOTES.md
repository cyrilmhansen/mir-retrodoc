# Public Release Notes

This repository is a public preservation and experimental rewrite workspace for
MIR.

The long-term project plan lives in `ROADMAP.md`.

## Current Public State

- `MIR-F0 v0` is implemented and documented as the baseline subset.
- The workspace now includes post-F0/F1 prototypes for lowered plans,
  optimization, RV32I assembly, dynamic JIT execution, and broader differential
  testing.
- `mircap`, `mirsem`, `mirc0`, `mirtool`, `mirspace`, `mirplan`, `mirrv32`, and
  `mirjit` all have passing test suites in the current workspace state.
- `mirspace` provides the indexed analysis layer used by `mirplan` and computes
  conservative static function effect summaries.
- `mirsem` trace snapshots now include effect counters for branches, direct
  call instructions, address computations, memory reads/writes, returns, traps,
  caller/callee edges, and per-function observations.
- `mirplan` provides deterministic compile-plan artifacts and text rendering.
- `mirtool analyze`, `mirtool trace-check`, `mirtool trace-cost`,
  `mirtool plan`, `mirtool lower`, and `mirtool diff-all` expose static
  analysis, trace-backed, lowered, and differential workflows through the CLI.
- `mirtool analyze --json` and `mirtool trace-check --json` provide the first
  machine-readable reflection reports.
- `mirtool cost` and `mirtool cost --json` expose conservative symbolic cost
  summaries over lowered plans, with cyclic CFGs marked unbounded/unknown.
- `mirtool trace-cost` and `mirtool trace-cost --json` compare symbolic cost
  summaries with observed `mirsem` counters and classify each unit as exact,
  within structural bound, exceeding the structural bound, or observation-only
  for cyclic CFGs.
- `i64` operations and byte memory operations are implemented across the
  interpreter and supported backend paths.
- `f32` and `f64` constants and arithmetic are implemented in `mircap`,
  `mirsem`, `mirc0`, and the C differential path; RV32/JIT float emission
  remains pending.
- GitHub Actions CI runs `./scripts/test-all.sh` and
  `./scripts/fmt-all.sh --check`.

## Public Boundaries

- Upstream MIR is referenced and credited separately in `ATTRIBUTION.md` and
  `docs/upstream-mir.md`.
- This repository does not vendor the upstream MIR source tree.
- Unsupported upstream MIR behavior is rejected or documented explicitly.

## F0 / F1 Boundary

- F0 is the frozen validated baseline subset.
- F1 now includes `mirspace`, `mirplan`, lowered C, optimization, RV32I, JIT
  demo execution, and differential tooling.
- The next recommended demo-facing feature is generating fixture families over
  increasing input sizes and using `trace-cost` to classify empirical growth,
  followed by deliberate float comparison and conversion specs plus an RV32FD
  versus soft-float backend decision.

## Validation Commands

```sh
./scripts/test-all.sh
./scripts/fmt-all.sh --check
```
