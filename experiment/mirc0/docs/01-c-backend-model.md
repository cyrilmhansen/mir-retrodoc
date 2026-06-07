# C Backend Model

This document describes how `mirc0` maps MIR-F0 constructs to standard, portable C code.

## Conventions & Mapping

### Value Types
- `TypeKind::I32` maps to `int32_t`
- `TypeKind::U32` maps to `uint32_t`
- `TypeKind::Addr32` maps to `uint32_t`
- `TypeKind::Void` maps to `void`

### Function Names and Signatures
MIR-F0 function names are prefixed with `mir_fn_` and their unique ID to avoid collisions (e.g. `mir_fn_1`).
Function parameters are mapped to arguments, and results are returned directly. Multiple results are not supported in `mirc0` v0.

### Control Flow
- MIR-F0 blocks become C labels (`block_<id>:`).
- Unconditional branch maps to `goto`.
- Conditional branch maps to `if (cond != 0) goto true_block; else goto false_block;`.

### Arithmetic
- Signed `i32` arithmetic maps to wrapping arithmetic via `uint32_t` casting, preventing C signed-overflow Undefined Behavior.
  E.g. `AddI32` becomes `(int32_t)((uint32_t)lhs + (uint32_t)rhs)`.
