# MIR-F0 v0 Status

This file is the implementation-facing status note for MIR-F0 v0 and the
post-F0 extensions now present in the workspace. It summarizes the current
target without replacing the full language contract in
`MIR-F0-v0-contract.md`.

## Objective

MIR-F0 v0 is the frozen first experimental subset. The current repository has
also moved into F1 prototype work. The baseline must provide:

- an immutable `mircap::ModuleImage` source of truth;
- conservative validation with explicit rejection of unsupported features;
- deterministic `mirsem` execution as the semantic oracle;
- `mirc0` C11 output that differentially matches `mirsem`;
- `mirtool` commands for validation, execution, binary flow, C output, and diffing;
- `mirspace` as the analysis-ready derived program representation.

MIR-F0 v0 is not upstream MIR. Unsupported upstream MIR behavior must not be
silently accepted.

Post-F0 additions currently include `i64`, byte memory operations, lowered-plan
artifacts, optimization, RV32I code generation, dynamic JIT demonstration, and
partial `f32`/`f64` constants/arithmetic in the semantic oracle.

## Included Crates

- `experiment/mircap`: schema, text fixture loader, Cap'n Proto binary flow,
  validation, and pretty printing.
- `experiment/mirsem`: deterministic interpreter, linear memory, traps, and
  trace snapshots.
- `experiment/mirc0`: C11 backend and differential execution tests.
- `experiment/mirtool`: developer CLI over the F0 pipeline.
- `experiment/mirspace`: dense indexed view over validated `ModuleImage`
  values, including stable-ID lookup maps and CFG edges.
- `experiment/mirplan`: deterministic compile-plan and lowered-program
  artifacts, plus local optimization.
- `experiment/mirrv32`: experimental RV32I backend over lowered programs.
- `experiment/mirjit`: experimental dynamic execution path for generated code.

## Completion Gate

The original F0 v0 gate is complete when all of the following hold:

- all documented supported opcodes have success or trap coverage;
- every documented trap has `mirsem` and differential C coverage where relevant;
- invalid fixtures cover unsupported type/opcode and structural rejection paths;
- trace snapshot fields used by F0 are documented and tested;
- `mirspace` imports valid fixtures, resolves dense indexes, and builds CFG
  predecessor/successor edges without mutating `ModuleImage`;
- `mirtool` smoke tests cover text, binary, run, trace, compile, and diff flows;
- all experiment crate test suites pass.

The current workspace extends that gate with `mirplan`, `mirrv32`, and `mirjit`
test coverage. Float fixtures currently validate and run in `mirsem`, while
`mirtool diff-all` skips them until C/RV32/JIT float emission exists.

## Explicit Non-Goals

- no full upstream MIR compatibility;
- no full host C ABI, varargs, long double, aggregate lowering, or indirect
  calls;
- no float comparisons, conversions, memory operations, C emission, RV32
  emission, or JIT FFI bridge;
- no RV32FD or soft-float backend design yet;
- no lazy basic-block versioning;
- no runtime code replacement or deoptimization.

## F1/F2 Handoff

F1 work is active. The most useful next F1 step is to complete the float
C/differential path for `f32`/`f64` constants and arithmetic, then decide the
RV32FD versus soft-float strategy separately. F2 remains reserved for counters,
code versions, and controlled replacement.
