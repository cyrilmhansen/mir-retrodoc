# Validation Rules

The first validation pass checks structural correctness for the immutable
module image.

## Implemented Rules

- format schema name is supported;
- format version is supported;
- type IDs are unique;
- symbol IDs are unique;
- function IDs are unique;
- block IDs are unique;
- instruction IDs are unique;
- unsupported MIR-F0 types are rejected;
- functions reference existing function symbols;
- function signature type references exist;
- functions reference existing blocks;
- blocks belong to their declared parent function;
- blocks reference existing instructions;
- every block has a terminator;
- the terminator is the final instruction in the block;
- no instruction appears after a terminator;
- branch targets exist and belong to the same function;
- direct calls reference existing functions;
- call argument and result counts match the callee signature;
- return value count matches the current function result count;
- opcode operand/result counts are checked for the supported subset.
- value type table length matches `value_count`;
- parameter types match the leading value table entries;
- integer opcode operand/result types are checked;
- memory opcode operand/result types are checked;
- data segments reference data symbols and have non-overflowing static ranges.

## Memory Validation

Static validation checks malformed memory instructions:

- `alloc` has one `addr32` result and two integer/immediate operands;
- `load_i32` has one `i32` result and one `addr32` operand;
- `load_u32` has one `u32` result and one `addr32` operand;
- `load_u8` has one `u32` result and one `addr32` operand;
- `store_i32` has no result, one `addr32` address, and one `i32` value;
- `store_u32` has no result, one `addr32` address, and one `u32` value;
- `store_u8` has no result, one `addr32` address, and one `u32` value;
- `addr_add` has one `addr32` result, one `addr32` base operand, and one `u32`
  offset operand;
- `data_addr` has one `addr32` result, one Symbol operand (referencing a Data symbol in a declared DataSegment), and one `u32` offset operand (immediate `ImmU32` or value `U32`). If the offset operand is a static immediate `ImmU32` constant, it must not exceed the referenced segment's length (bytes size + zero fill); otherwise, it is a validation error.
- i64 memory forms are rejected through unsupported type/opcode validation.

`branch_if` validation requires one `u32` condition operand and two explicit
same-function block targets. There is no implicit false-target fallthrough.

Execution traps are not validation errors. Out-of-bounds access (including dynamic `data_addr` offsets exceeding segment length), heap/stack collision, out-of-memory, and misalignment belong to the executor.

## Unsigned Arithmetic & Comparison Validation

Static type safety checks enforce the following:
- `add_u32`, `sub_u32`, `mul_u32` require two `u32` input operands and one `u32` result.
- `eq_u32`, `ne_u32`, `lt_u32`, `le_u32`, `gt_u32`, `ge_u32` require two `u32` input operands and one `u32` result (representing 0 or 1).
- No implicit casting is permitted between `i32`, `u32`, and `addr32`.

## Error Model

Errors are structured as:

- kind;
- entity reference;
- optional source span;
- human-readable message.

Implemented error kinds include invalid format, unsupported version, duplicate
ID, missing reference, wrong parent, invalid terminator, type mismatch,
unsupported feature, malformed operand, malformed function signature, and
unresolved symbol.

## Open Questions

- Should value/register definitions be type-tracked in MIR-F0?
- Should validators derive block predecessor/successor indexes?
- Should unsupported features produce schema-level tags or loader-level errors?
- Should data segments have stable IDs separate from `SymbolId`?
