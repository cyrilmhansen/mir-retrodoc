# Experimental Rewrite Overview

This directory documents the experimental rewrite track. It is separate from
the preservation-first MIR retrospective documentation and from the
design-perspective notes.

The first module is `experiment/mircap`, a Rust crate defining a Cap'n Proto
based immutable `ModuleImage` for `MIR-F0`.

`MIR-F0` is not full MIR. Unsupported upstream MIR features must not be silently
accepted.

## Current Scope

- immutable module image;
- stable IDs;
- explicit functions, blocks, instructions, and operands;
- conservative validation;
- deterministic interpreter/oracle;
- C backend and differential tests;
- indexed analysis view;
- deterministic compile-plan and lowered artifacts;
- local lowered-plan optimization;
- RV32I backend experiments;
- dynamic JIT demo path;
- fixture-based tests.

## Current Boundaries

The rewrite remains intentionally much smaller than upstream MIR. Host C ABI
coverage, varargs, aggregates, indirect calls, long double, lazy basic-block
versioning, full runtime code replacement, and ECS workspace conversion remain
out of scope.

Floating point has started but is partial: `f32` and `f64` constants and
arithmetic are validated and executable in `mirsem`; C/RV32/JIT float emission,
comparisons, conversions, and float memory operations are still future work.
