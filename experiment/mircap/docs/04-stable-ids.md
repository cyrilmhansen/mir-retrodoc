# Stable IDs

Stable IDs are stored in the module image and are intended to survive
serialization.

## Stored Stable IDs

- `FunctionId`
- `BlockId`
- `InstructionId`
- `ValueId`
- `TypeId`
- `SymbolId`
- `SourceSpanId`

These IDs are suitable for trace snapshots, diagnostics, pretty-printing, and
future tooling references.

## Dense Indexes

Dense indexes are derived by the loader or validator. They are implementation
details and should not be serialized as semantic identity.

Examples:

- function ID to vector index;
- block ID to vector index;
- instruction ID to vector index;
- per-function block order;
- per-block instruction order.

## Rule

Do not confuse stable IDs with dense indexes. Stable IDs are part of the module
image. Dense indexes are loader/compiler/runtime conveniences.

