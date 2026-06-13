# Stable IDs vs Dense Indexes

`mirspace` distinguishes between stable identifiers and local dense indexes to support compiler analysis alongside diagnostic tracing.

## Stable IDs
- **Definition**: Stable identifier newtypes (e.g. `FunctionId`, `BlockId`) originating from the validated `ModuleImage`.
- **Purpose**: Ideal for trace serialization, JIT debugging reports, compiler logs, and external interactive editor references.

## Dense Indexes
- **Definition**: Local type-safe index wrappers (e.g. `FunctionIx`, `BlockIx`) wrapping `usize`.
- **Purpose**: Used for direct lookup into top-level collections in `ProgramSpace`. This avoids expensive map lookups, lookup indirections, and reference borrow issues during compiler analysis passes.
