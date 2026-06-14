# mir-retrodoc

`mir-retrodoc` is a preservation and experimental rewrite workspace for studying
Vladimir Makarov's MIR lightweight JIT compiler.

The project has two deliberately separate tracks:

- retrospective documentation of upstream MIR behavior, based on preserved
  source and project material;
- MIR-inspired experiments that define and test a small, explicit subset called
  MIR-F0, then build compiler-facing F1 prototypes on top of it.

This repository is not upstream MIR, and MIR-F0 is not full MIR. Unsupported
upstream MIR behavior is rejected or documented explicitly instead of being
silently accepted.

Upstream MIR lives at <https://github.com/vnmakarov/mir> and is credited in
`docs/upstream-mir.md`.

The high-level project plan is tracked in `ROADMAP.md`. The current public
snapshot is tracked in `RELEASE-NOTES.md`.

## Repository Layout

- `ROADMAP.md`: project phases, current F0/F1 boundary, and deferred work.
- `RELEASE-NOTES.md`: current public repository snapshot and validation entry
  points.
- `docs/`: preservation-first notes about MIR structure, APIs, parser behavior,
  JIT/runtime topics, and open questions.
- `docs/design-perspectives/`: exploratory future-design notes. These are not
  claims about upstream MIR intent.
- `docs/experimental-rewrite/`: MIR-F0 language contract, compliance notes, and
  current F0/F1 status.
- `docs/upstream-mir.md`: upstream MIR attribution and source-reference policy.
- `experiment/mircap`: immutable MIR-F0 module image, validation, fixture loader,
  and Cap'n Proto roundtrip support.
- `experiment/mirsem`: deterministic MIR-F0 interpreter/oracle with trace
  snapshots and execution traps.
- `experiment/mirc0`: C11 backend tested differentially against `mirsem`.
- `experiment/mirtool`: developer CLI for
  validate/run/encode/decode/compile/diff/plan/lower/RV32I flows.
- `experiment/mirspace`: dense indexed analysis view over validated
  `mircap::ModuleImage` values.
- `experiment/mirplan`: deterministic compile-plan artifacts consumed from
  `mirspace`, used to stabilize future compiler inputs before code generation.
- `experiment/mirrv32`: experimental RV32I backend over lowered plans.
- `experiment/mirjit`: experimental dynamic execution path for generated code.

## Current Milestone

MIR-F0 v0 is the baseline frozen subset. The current workspace has moved into
post-F0/F1 prototype territory: it supports integer, address, memory,
control-flow, direct-call, return, trap, `i64`, byte-memory, lowered-plan,
optimization, RV32I, JIT-demo, and differential-test workflows.

Floating point is underway. `f32` and `f64` constants and arithmetic are
validated and executable in `mirsem`, but C/RV32/JIT float emission is still the
next implementation target.

## Quick Demo

Run the project tour:

```sh
./scripts/demo.sh
```

For a non-interactive run:

```sh
./scripts/demo.sh --no-pause
```

For environments without a host C compiler:

```sh
./scripts/demo.sh --no-cc
```

See `docs/demo.md` for the demo narrative and expected outputs.

## Running Tests

Preferred commands:

```sh
./scripts/test-all.sh
./scripts/fmt-all.sh
./scripts/fmt-all.sh --check
```

Each experiment crate is standalone:

```sh
cd experiment/mircap && cargo test
cd ../mirsem && cargo test
cd ../mirc0 && cargo test
cd ../mirtool && cargo test
cd ../mirspace && cargo test
cd ../mirplan && cargo test
cd ../mirrv32 && cargo test
cd ../mirjit && cargo test
```

The `mirc0` and `mirtool` differential tests use the host C compiler `cc` when
available.

`scripts/test-all.sh` runs the same crate tests in a fixed order and fails fast.
`scripts/fmt-all.sh --check` verifies rustfmt output without rewriting files.

## License

This repository is licensed under the MIT License. See `LICENSE`.

Upstream MIR is also MIT licensed and is credited separately in
`ATTRIBUTION.md` and `docs/upstream-mir.md`.

## Public Repository Hygiene

Build products and temporary differential-test files are ignored through the
root `.gitignore` and crate-local `.gitignore` files. Source, tests, fixtures,
schemas, lockfiles, scripts, and documentation are intended to be versioned.

See `ROADMAP.md` for the high-level project plan and `RELEASE-NOTES.md` for the
current public repository snapshot.
