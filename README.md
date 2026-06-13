# mir-retrodoc

`mir-retrodoc` is a preservation and experimental rewrite workspace for studying
Vladimir Makarov's MIR lightweight JIT compiler.

The project has two deliberately separate tracks:

- retrospective documentation of upstream MIR behavior, based on preserved
  source and project material;
- MIR-inspired experiments that define and test a small, explicit subset called
  MIR-F0.

This repository is not upstream MIR, and MIR-F0 is not full MIR. Unsupported
upstream MIR behavior is rejected or documented explicitly instead of being
silently accepted.

Upstream MIR lives at <https://github.com/vnmakarov/mir> and is credited in
`docs/upstream-mir.md`.

## Repository Layout

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
- `experiment/mirtool`: developer CLI for validate/run/encode/decode/compile/diff
  flows.
- `experiment/mirspace`: dense indexed analysis view over validated
  `mircap::ModuleImage` values.
- `experiment/mirplan`: deterministic compile-plan artifacts consumed from
  `mirspace`, used to stabilize future compiler inputs before code generation.

## Current Milestone

MIR-F0 v0 is the current frozen subset. It supports a small set of integer,
address, memory, control-flow, direct-call, return, and trap operations. The
current status gate is tracked in
`docs/experimental-rewrite/F0-status.md`.

F1 work starts from `mirspace`: it adds compiler-facing analysis structures
without expanding the language surface first.

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

See `RELEASE-NOTES.md` for the current public repository snapshot and
validation entry points.
