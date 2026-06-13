# mirspace Data Model

`ProgramSpace` is structured around dense lookup tables of records, maximizing traversal efficiency.

## 1. Top-Level Tables
```rust
pub struct ProgramSpace {
    pub name: String,
    pub functions: Vec<FunctionRec>,
    pub blocks: Vec<BlockRec>,
    pub instructions: Vec<InstructionRec>,
    pub operands: Vec<OperandRec>,
    pub values: Vec<ValueRec>,
    pub edges: Vec<EdgeRec>,
    pub data_segments: Vec<DataSegmentRec>,
    pub symbols: Vec<SymbolRec>,
    pub maps: IdMaps,
}
```

## 2. Scoping of Values
- **Function Scoping**: `ValueId` values (e.g. `%0`, `%1`) are scoped strictly per function in MIR-F0.
- **Global Mapping**: In `IdMaps`, values are indexed by a compound key `(FunctionId, ValueId)` mapping to a global dense `ValueIx`.
- **Value Roles**:
  - `Parameter`: The value maps to one of the input parameters of the parent function.
  - `Local`: The value maps to an internal or instruction-defined local variable.
