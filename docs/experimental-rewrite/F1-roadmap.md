# MIR-F1 Roadmap

MIR-F1 is the next practical subset after MIR-F0 v0. Its purpose is to move from
"validated execution model" to "compiler-ready internal representation" without
expanding the MIR-F0 language surface prematurely.

## F1 Direction

F1 should strengthen the middle layer:

- use `mirspace::ProgramSpace` as the canonical analysis view over
  `mircap::ModuleImage`;
- keep `mirsem` as the semantic oracle;
- keep `mirc0` differential testing as the correctness discipline;
- produce deterministic analysis and planning artifacts before adding new
  execution targets.

## First Technical Axis

The first F1 axis is `mirspace` analysis:

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

## Deferred From Early F1

These remain intentionally deferred until the analysis layer is firmer:

- `i64` helper lowering;
- floating point;
- host C ABI support;
- RISC-V32 backend work;
- optimization;
- runtime replacement, deoptimization, or lazy basic-block versioning.

## F1 Milestones

1. Add value def-use indexing in `mirspace`.
2. Add a simple deterministic block traversal suitable for compiler planning.
3. Add a compile-plan artifact that lists functions, blocks, values, calls, and
   memory operations without generating code.
4. Add a backend-facing projection over the compile plan without choosing a
   target, register model, or optimizer.
5. Differentially verify that planning does not mutate `ModuleImage` and stays
   stable across text and Cap'n Proto load paths.
6. Expose planning and lowering artifacts through read-only CLI inspection.
7. Prove the lowered contract with an experimental C backend path before
   choosing a new target.
8. Only then choose the first target-facing F1 feature.

## Exit Criteria

F1 is ready to move toward target work when:

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
