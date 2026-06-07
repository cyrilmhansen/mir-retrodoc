# Stable IDs

Stable IDs are part of the module image:

- `FunctionId`
- `BlockId`
- `InstructionId`
- `ValueId`
- `TypeId`
- `SymbolId`
- `SourceSpanId`

They are suitable for diagnostics, trace snapshots, and tooling references.

Dense indexes are derived loader state. They may be used for efficient
validation and compilation, but should not be treated as serialized identity.

## Rule

Stable IDs survive serialization. Dense indexes are disposable implementation
details.

