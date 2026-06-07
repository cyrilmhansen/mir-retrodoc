# MIRCap Module Image

`ModuleImage` is the immutable loaded bytecode/module image for MIR-F0.

Cap'n Proto is used as the planned serialization format. The current Rust crate
contains a project-owned API so future code does not depend directly on raw
generated Cap'n Proto accessors.

## Chosen Structure

The recommended schema structure is table/range based rather than deeply
nested:

- top-level type table;
- top-level symbol table;
- top-level function table;
- top-level block table;
- top-level instruction table;
- top-level operand table.

Functions reference block ranges. Blocks reference instruction ranges.
Instructions reference operand ranges.

## Rationale

The table/range model is better for load-time indexing, validation, later
compiler passes, and future ECS/SoA conversion. It also keeps stable IDs usable
for trace snapshots without mutating the image.

Nested objects are simpler initially, but make global indexing and stable ID
validation less explicit.

## Exclusions

Runtime counters, compiled-code addresses, editor state, interpreter stacks,
and compiler temporaries are not part of the immutable image.

