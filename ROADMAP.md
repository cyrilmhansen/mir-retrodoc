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
- `mirspace` computes conservative static function effect summaries for
  allocation, memory effects, trap possibility, direct calls, CFG acyclicity,
  trivial termination, and pure-candidate detection;
- `mirsem` traces now expose effect counters and observed caller/callee edges
  that can be compared with static summaries;
- `mirplan` produces deterministic compile-plan and lowered artifacts;
- lowered C emission is implemented while preserving the stable `ModuleImage`
  C compiler path;
- optimization exists for local constant propagation/folding and dead-code
  elimination on lowered plans;
- `mirtool analyze`, `mirtool trace-check`, `mirtool plan`, `mirtool lower`,
  and `mirtool diff-all` expose inspection and differential workflows;
- `mirtool analyze --json` and `mirtool trace-check --json` expose the first
  machine-readable reflection contract for tests, demos, and later tooling;
- `mirrv32` emits RV32I assembly for the supported integer/address/memory
  subset, including `i64` lowering through register-pair style codegen;
- `mirjit` demonstrates dynamic in-process execution through generated RV32I
  artifacts and host loading.

Detailed F1 scope and exit criteria live in
`docs/experimental-rewrite/F1-roadmap.md`.

## Phase 3: MIR-F2 Reflection, Contracts, And Runtime Intelligence

Purpose: turn the validated IR, interpreter oracle, lowered plans, and trace
snapshots into a reflective runtime research platform. This phase should make
program behavior inspectable and partially provable before attempting aggressive
runtime replacement or speculative optimization.

Status: first static and trace-backed slice started through `mirspace` effect
summaries, `mirsem` effect counters and call-edge counters, `mirtool analyze`,
`mirtool trace-check`, and JSON output for both commands. The broader
conceptual starting point remains
`docs/design-perspectives/02-runtime-introspection-and-tracing.md`.

Target capabilities:

- expose structured runtime metadata for modules, functions, blocks, values,
  data segments, lowered plans, compiled code ranges, and execution counters;
- add first-class function property summaries such as `pure`, `reads-memory`,
  `writes-memory`, `allocates`, `may-trap`, `may-call`, and `may-not-return`;
- let `mirsem` act as a semantic checker for those summaries by recording
  memory effects, allocation events, caller/callee edges, traps, fuel use, and
  return paths;
- distinguish proven facts from observed facts. For example, "does not allocate
  in this run" is weaker than "cannot allocate for any input";
- add simple proof-oriented analyses for narrow cases: no allocation, no memory
  writes, direct-call-only purity, bounded loop trip counts, acyclic CFGs, and
  guaranteed termination for straight-line or statically bounded code;
- export machine-readable reflection data through `mirtool`, suitable for
  tests, demos, and later IDE/runtime tooling;
- keep JSON reflection output stable enough for scripts while reserving the
  human-readable text output for demos and quick inspection;
- keep metaprogramming explicit and constrained: generated or transformed
  modules must pass the same validation, pretty-printing, binary roundtrip, and
  differential checks as hand-written fixtures.

Runtime performance monitoring:

- collect function, block, and edge execution counts;
- measure wall-clock and instruction/fuel cost per function or region;
- count allocations, memory reads/writes, traps, calls, and backend transitions;
- track generated C/RV32/JIT code size and compilation time where available;
- support multiple instrumentation levels so measurement overhead is explicit.

Complexity analysis:

- compute symbolic cost summaries over the lowered plan for simple patterns,
  starting with straight-line code, acyclic CFGs, and loops with statically
  visible bounds;
- report costs in abstract units first: instruction count, branch count, memory
  access count, allocation count, and call count;
- compare symbolic predictions with runtime measurements from `mirsem` traces
  and backend differential runs;
- classify empirical growth by running generated fixture families over multiple
  input sizes, then fitting simple families such as constant, linear,
  log-linear, quadratic, and exponential;
- always report confidence and limits. Complexity claims should say whether
  they are proven from IR structure, inferred from bounded symbolic analysis, or
  measured empirically.

## Current Recommended Next Step

The demo now has a coherent story from validation to interpretation, static
effect analysis, trace-backed call-edge checking, machine-readable JSON
reports, lowering, C differential checks, Cap'n Proto serialization, float
arithmetic, traps, and RV32I output. The best next demo-facing step is to add
the first symbolic cost summaries:

- summarize straight-line and acyclic lowered plans in abstract units:
  instructions, branches, calls, memory reads/writes, allocations, and traps;
- expose that cost summary in text and JSON so it can be compared with
  `mirsem` trace counters;
- keep float expansion deliberate by specifying comparisons, conversions, and
  the RV32FD versus soft-float backend decision separately.

This keeps the public demo concrete while moving toward the F2 reflection and
runtime-intelligence vision.

## Deferred Work

The following remain out of early F1 until the analysis and planning boundary is
stable:

- host C ABI and varargs;
- aggregate lowering;
- float comparisons and integer/float conversions;
- float RV32/JIT coverage beyond constants and arithmetic;
- RV32FD hardware floating-point or soft-float helper design;
- fantasy-computer target work;
- lazy basic-block versioning;
- runtime code replacement or deoptimization.
- advanced reflection, metaprogramming, symbolic complexity analysis, and
  empirical complexity classification.

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
