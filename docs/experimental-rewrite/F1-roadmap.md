# MIR-F1 Roadmap

MIR-F1 is the practical subset after MIR-F0 v0. Its purpose is to move from
"validated execution model" to "compiler-ready internal representation" and
then prove that representation through backend and differential workflows.

## F1 Direction

F1 strengthens the middle layer:

- use `mirspace::ProgramSpace` as the canonical analysis view over
  `mircap::ModuleImage`;
- keep `mirsem` as the semantic oracle;
- keep `mirc0` differential testing as the correctness discipline;
- produce deterministic analysis and planning artifacts before adding new
  execution targets.

## Implemented Technical Axis

The first F1 axis, `mirspace` plus `mirplan`, is implemented:

- def-use indexing for values;
- stable block and instruction traversal utilities;
- CFG-derived analyses needed by a future baseline compiler;
- deterministic compile-plan data structures in `experiment/mirplan` that can
  be tested before code generation exists;
- a target-neutral lowering projection that makes value reads, value writes,
  data segments, branches, calls, and memory operations explicit without
  generating code;
- CLI inspection through `mirtool plan` and `mirtool lower`.
- experimental backend validation through `mirc0::compile_lowered`, while
  keeping the stable `ModuleImage` compiler path unchanged.

F1 has also grown beyond analysis:

- local optimization over lowered programs;
- `mirtool diff-all` for native, upstream MIR, and RV32I differential checks;
- RV32I assembly generation in `mirrv32`;
- dynamic execution demonstration in `mirjit`;
- `i64` support across validation, interpretation, C, RV32I, and upstream diff
  paths.

## Deferred Or Partial From F1

These remain intentionally deferred or partial:

- float C emission and differential testing;
- float comparisons, conversions, memory operations, and backend strategy;
- host C ABI support;
- RV32FD or soft-float backend work;
- advanced reflection, metaprogramming, function-effect contracts, termination
  proofs, and symbolic/empirical complexity analysis;
- runtime replacement, deoptimization, or lazy basic-block versioning.

## F1 Milestones

1. Add value def-use indexing in `mirspace`. Done.
2. Add a simple deterministic block traversal suitable for compiler planning.
   Done.
3. Add a compile-plan artifact that lists functions, blocks, values, calls, and
   memory operations without generating code.
   Done.
4. Add a backend-facing projection over the compile plan without choosing a
   target, register model, or optimizer.
   Done.
5. Differentially verify that planning does not mutate `ModuleImage` and stays
   stable across text and Cap'n Proto load paths.
   Done.
6. Expose planning and lowering artifacts through read-only CLI inspection.
   Done.
7. Prove the lowered contract with an experimental C backend path before
   choosing a new target.
   Done for the integer/address/memory subset.
8. Choose the first target-facing F1 feature.
   Done for RV32I.
9. Complete the first float differential path.
   Current recommended next milestone.

## Exit Criteria

The original F1 target-readiness gate is satisfied for integer/address/memory
work when:

- `mirspace` exposes tested analysis views for values, instructions, blocks, and
  calls;
- `mirplan` produces deterministic planning artifacts over representative F0
  fixtures;
- `mirplan` exposes a tested target-neutral lowering projection with module data
  segment summaries;
- `mirplan` artifacts are identical across text and Cap'n Proto load paths;
- `mirtool` exposes both plan and lower inspection paths;
- `mirc0` can experimentally compile representative F0 fixtures from
  `LoweredProgram`;
- all analysis output is deterministic;
- all F0 tests remain green;
- the planned baseline compiler input is documented and covered by fixtures.

The remaining F1 readiness gap is float parity. `f32` and `f64` constants and
arithmetic already validate and execute in `mirsem`; they should next be
emitted by `mirc0`, compared by `mirtool diff`, and included in `diff-all`.

## F2 Handoff: Runtime Intelligence

After F1 has stable lowered plans and differential backends, the next research
track is not just more opcodes. F2 should use the existing deterministic IR as a
base for runtime intelligence:

- static effect summaries: purity, memory reads/writes, allocation behavior,
  trap behavior, call behavior, and obvious non-termination risks;
- `mirsem` trace validation for those summaries, separating proven properties
  from observations gathered during one execution;
- symbolic cost summaries for simple CFGs and statically bounded loops;
- empirical complexity measurement over generated input-size families;
- reflective `mirtool` output that exposes these facts in a machine-readable
  form for tests, demos, and future tooling.
