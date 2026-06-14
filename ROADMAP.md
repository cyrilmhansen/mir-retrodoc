# Project Roadmap

`mir-retrodoc` has two related tracks: preservation-oriented documentation of
upstream MIR and MIR-inspired experimental implementation work.

This roadmap is the high-level project map. `RELEASE-NOTES.md` records the
current public snapshot, while the detailed F0/F1 contracts live under
`docs/experimental-rewrite/`.

## Phase 0: Public Preservation

Purpose: document upstream MIR behavior, source structure, and design questions
without claiming ownership of upstream MIR.

Status:

- upstream MIR is credited in `ATTRIBUTION.md` and `docs/upstream-mir.md`;
- preservation notes live under `docs/`;
- exploratory future-design notes live under `docs/design-perspectives/`;
- this repository does not vendor the upstream MIR source tree.

## Phase 1: MIR-F0 v0 And Post-F0 Extensions

Purpose: freeze a small, explicit MIR-inspired subset with deterministic
validation, execution, C output, and CLI workflows.

Status:

- implemented as the original frozen subset, then extended experimentally;
- documented in `docs/experimental-rewrite/MIR-F0-v0-contract.md`;
- tracked operationally in `docs/experimental-rewrite/F0-status.md`;
- covered by the `mircap`, `mirsem`, `mirc0`, `mirtool`, `mirspace`,
  `mirplan`, `mirrv32`, and `mirjit` test suites.

The currently implemented language surface includes integer, address, memory,
control-flow, direct-call, return, trap, `i64`, and byte-memory operations.
Floating-point support has started: `f32` and `f64` constants and arithmetic are
validated and executable in `mirsem`, but are not yet emitted by the C, RV32, or
JIT backends.

Unsupported upstream MIR features are rejected or documented explicitly.

## Phase 2: MIR-F1 Analysis, Planning, And Backends

Purpose: prepare a compiler-facing internal representation before expanding the
language surface or adding new targets.

Status:

- `mirspace::ProgramSpace` is the indexed analysis view;
- `mirplan` produces deterministic compile-plan and lowered artifacts;
- lowered C emission is implemented while preserving the stable `ModuleImage`
  C compiler path;
- optimization exists for local constant propagation/folding and dead-code
  elimination on lowered plans;
- `mirtool plan`, `mirtool lower`, and `mirtool diff-all` expose inspection and
  differential workflows;
- `mirrv32` emits RV32I assembly for the supported integer/address/memory
  subset, including `i64` lowering through register-pair style codegen;
- `mirjit` demonstrates dynamic in-process execution through generated RV32I
  artifacts and host loading.

Detailed F1 scope and exit criteria live in
`docs/experimental-rewrite/F1-roadmap.md`.

## Current Recommended Next Step

The best next demo-facing step is to complete the first floating-point
differential path:

- add `mirc0` C output for `f32` and `f64` constants and arithmetic;
- print float results in a deterministic bit-pattern-friendly form;
- include the float fixtures in `mirtool diff` and `mirtool diff-all` instead
  of skipping them;
- extend the demo to show the float oracle and C backend agreeing.

This keeps the project easy to demonstrate while postponing the harder RV32FD
or soft-float backend decision.

## Deferred Work

The following remain out of early F1 until the analysis and planning boundary is
stable:

- host C ABI and varargs;
- aggregate lowering;
- float comparisons and integer/float conversions;
- float C/RV32/JIT coverage beyond constants and arithmetic;
- RV32FD hardware floating-point or soft-float helper design;
- fantasy-computer target work;
- lazy basic-block versioning;
- runtime code replacement or deoptimization.

## Quality Gates

Current validation entry points:

```sh
./scripts/test-all.sh
./scripts/fmt-all.sh --check
```

GitHub Actions runs both commands on push and pull request.

## Demo Path

The current public demo is documented in `docs/demo.md` and can be run with:

```sh
./scripts/demo.sh
```
