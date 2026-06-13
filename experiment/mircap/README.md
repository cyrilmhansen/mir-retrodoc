# mircap

`mircap` is the first experimental module for a MIR-inspired rewrite. It defines
the immutable loaded bytecode/module image for the minimal `MIR-F0` subset.

This crate is separate from:

- upstream MIR source references, credited in `../../docs/upstream-mir.md`;
- retrospective documentation under `docs/`;
- exploratory design-perspective notes under `docs/design-perspectives/`.

`MIR-F0` is not full MIR. Compatibility means compatibility with the explicitly
documented subset only. Unsupported upstream MIR features must not be silently
accepted.

## Current Status

The crate contains:

- a Cap'n Proto schema draft in `schema/mircap.capnp`;
- a Rust-owned API around a `ModuleImage`;
- a first validation pass;
- a minimal MIR-F0 linear-memory image model with data segments and memory
  instruction validation;
- text fixtures used by tests.

Generated Cap'n Proto bindings are not wired yet. The current tests load a small
line-oriented `.mircap.txt` fixture format into the same Rust `ModuleImage`
model that Cap'n Proto decoding should later populate.

## Non-Goals

This module does not implement an interpreter, compiler, optimizer, RISC-V32
backend, ECS workspace, runtime tracing, or code replacement.

Execution resource limits, out-of-bounds memory traps, heap/stack collision,
and alignment traps belong to execution modules such as `mirsem`, not to the
immutable `ModuleImage`.
