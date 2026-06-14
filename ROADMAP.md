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

## Phase 1: MIR-F0 v0

Purpose: freeze a small, explicit MIR-inspired subset with deterministic
validation, execution, C output, and CLI workflows.

Status:

- implemented as the current frozen subset;
- documented in `docs/experimental-rewrite/MIR-F0-v0-contract.md`;
- tracked operationally in `docs/experimental-rewrite/F0-status.md`;
- covered by the `mircap`, `mirsem`, `mirc0`, `mirtool`, and `mirspace` test
  suites.

F0 remains intentionally narrow. Unsupported upstream MIR features are rejected
or documented explicitly.

## Phase 2: MIR-F1 Analysis And Planning

Purpose: prepare a compiler-facing internal representation before expanding the
language surface or adding new targets.

Current F1 direction:

- use `mirspace::ProgramSpace` as the indexed analysis view;
- keep `mirsem` as the semantic oracle;
- keep `mirc0` differential testing as the correctness discipline;
- use `mirplan` to produce deterministic compile-plan artifacts;
- use `mirplan` lowering projections as backend-facing, target-neutral input;
- expose inspection through `mirtool plan` and `mirtool lower`.

Detailed F1 scope and exit criteria live in
`docs/experimental-rewrite/F1-roadmap.md`.

## Deferred Work

The following remain out of early F1 until the analysis and planning boundary is
stable:

- `i64` helper lowering;
- floating point;
- host C ABI and varargs;
- aggregate lowering;
- RISC-V32 or fantasy-computer target work;
- optimization;
- lazy basic-block versioning;
- runtime code replacement or deoptimization.

## Quality Gates

Current validation entry points:

```sh
./scripts/test-all.sh
./scripts/fmt-all.sh --check
```

GitHub Actions runs both commands on push and pull request.
