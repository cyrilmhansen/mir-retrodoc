# MIR-F0 v0 Status

This file is the implementation-facing completion checklist for MIR-F0 v0.
It summarizes the current target without replacing the full language contract
in `MIR-F0-v0-contract.md`.

## Objective

MIR-F0 v0 is the frozen first experimental subset. It must provide:

- an immutable `mircap::ModuleImage` source of truth;
- conservative validation with explicit rejection of unsupported features;
- deterministic `mirsem` execution as the semantic oracle;
- `mirc0` C11 output that differentially matches `mirsem`;
- `mirtool` commands for validation, execution, binary flow, C output, and diffing;
- `mirspace` as the analysis-ready derived program representation.

MIR-F0 v0 is not upstream MIR. Unsupported upstream MIR behavior must not be
silently accepted.

## Included Crates

- `experiment/mircap`: schema, text fixture loader, Cap'n Proto binary flow,
  validation, and pretty printing.
- `experiment/mirsem`: deterministic interpreter, linear memory, traps, and
  trace snapshots.
- `experiment/mirc0`: C11 backend and differential execution tests.
- `experiment/mirtool`: developer CLI over the F0 pipeline.
- `experiment/mirspace`: dense indexed view over validated `ModuleImage`
  values, including stable-ID lookup maps and CFG edges.

## Completion Gate

F0 v0 is complete when all of the following hold:

- all documented supported opcodes have success or trap coverage;
- every documented trap has `mirsem` and differential C coverage where relevant;
- invalid fixtures cover unsupported type/opcode and structural rejection paths;
- trace snapshot fields used by F0 are documented and tested;
- `mirspace` imports valid fixtures, resolves dense indexes, and builds CFG
  predecessor/successor edges without mutating `ModuleImage`;
- `mirtool` smoke tests cover text, binary, run, trace, compile, and diff flows;
- all experiment crate test suites pass.

## Explicit Non-Goals

- no full upstream MIR compatibility;
- no host C ABI, varargs, floating point, long double, aggregate lowering, or
  indirect calls;
- no RISC-V32 backend;
- no optimizer;
- no lazy basic-block versioning;
- no runtime code replacement or deoptimization.

## F1/F2 Handoff

F1 work can begin after the F0 gate passes. Likely F1 candidates are helper
lowering for larger integer operations, richer trace export, a real baseline
target, and a tighter integration between `mirspace` and future compiler passes.
F2 remains reserved for counters, code versions, and controlled replacement.
