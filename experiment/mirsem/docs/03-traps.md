# Traps

`mirsem` keeps error categories distinct:

- `mircap` validation errors;
- unsupported MIR-F0 features;
- execution traps;
- internal evaluator errors.

## Structured Traps

- `StackOverflow`
- `FuelExhausted`
- `ExplicitTrap`
- `UnsupportedInstruction`
- `UnsupportedType`
- `InvalidBlock`
- `InvalidInstruction`
- `CallArityMismatch`
- `ReturnArityMismatch`
- `UninitializedValue`
- `OutOfMemory`
- `HeapStackCollision`
- `OutOfBoundsLoad`
- `OutOfBoundsStore`
- `MisalignedLoad`
- `MisalignedStore`
- `AddressOverflow`

Memory-related traps are active for `alloc`, `load_i32`, `load_u32`,
`store_i32`, `store_u32`, and `addr_add`.
