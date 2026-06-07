# mirsem Overview

`mirsem` is a strict MIR-F0 reference evaluator. It consumes
`mircap::ModuleImage`, requires successful `mircap` validation, executes the
supported MIR-F0 subset deterministically, and emits a separate trace snapshot.

It is an oracle for future baseline compiler work. It is not a production
interpreter.

## Boundaries

- no native compiler;
- no optimizer;
- no RISC-V32 backend;
- no host C ABI;
- no dynamic linking to host C symbols;
- no lazy JIT or lazy basic-block versioning;
- no code replacement;
- no ECS/live IDE workspace.

Trace counters and runtime observations are stored in `mirsem`, not in the
immutable `mircap::ModuleImage`.

