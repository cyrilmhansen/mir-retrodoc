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
- `mirspace` provides the indexed analysis layer used by `mirplan`.
- `mirplan` provides deterministic compile-plan artifacts and text rendering.
- `mirtool plan`, `mirtool lower`, and `mirtool diff-all` expose analysis,
  lowered, and differential workflows through the CLI.
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
- The next recommended demo-facing feature is a deliberate float comparison and
  conversion spec, followed by an RV32FD versus soft-float backend decision.

## Validation Commands

```sh
./scripts/test-all.sh
./scripts/fmt-all.sh --check
```
