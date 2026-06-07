# mircap Overview

`mircap` is an experimental rewrite module. It is not upstream MIR and is not
part of the preservation archive.

The module defines `ModuleImage`: an immutable loaded program image for
`MIR-F0`, a minimal MIR-inspired subset. The image is intended to be serialized
with Cap'n Proto and loaded into a small project-owned Rust API.

## Boundaries

- Cap'n Proto is the immutable module image.
- Runtime counters and traces are not stored in the module image.
- Compiled-code addresses are not stored in the module image.
- Editor-only mutable state is not stored in the module image.
- ECS/SoA workspaces are future dynamic representations, not this module.

## Current Implementation

The Cap'n Proto schema exists in `schema/mircap.capnp`. Rust generated bindings
are not wired yet. The current crate uses text fixtures to populate the same
`ModuleImage` API that Cap'n Proto decoding should later produce.

The current image includes the minimal memory constructs needed by MIR-F0:
`addr32`, module-level data segments, and static validation for `alloc`,
`load_i32`, `load_u32`, `store_i32`, `store_u32`, and `addr_add`.

## Open Questions

- Which Cap'n Proto generator should be used in the build?
- Should fixtures eventually be binary Cap'n Proto messages?
- Should string/BASIC-oriented byte operations add `load_u8` and `store_u8`?
