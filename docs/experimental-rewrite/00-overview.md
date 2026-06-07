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
- fixture-based tests.

## Out Of Scope

Interpreter, compiler, optimizer, RISC-V32 backend, runtime tracing, code
replacement, and ECS workspace conversion are future modules.

