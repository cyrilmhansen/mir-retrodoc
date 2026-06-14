# MIR-F0 v0 Compliance Matrix

This compliance matrix lists all stable test fixtures located under `experiment/mircap/tests/fixtures/`, mapping their validation outcome, interpreter outcome, transpile outcome, C compilation, and differential matching status.

## 1. Valid/Success Fixtures

These fixtures represent stable valid images. Most load, validate, transpile,
compile, and execute to completion. Float fixtures currently validate and run in
`mirsem`, but are intentionally skipped by C/RV32/upstream differential paths
until backend float emission exists.

| Fixture Path | `mircap` Load / Validate | `mirsem` Execution Result | `mirc0` Transpiled Execution Result | Strict C11 Compile | Differential Match |
|---|---|---|---|---|---|
| `valid_const_return.mircap.txt` | Accepts | `I32(42)` | `Result: 42` | Passes | Yes |
| `valid_arithmetic.mircap.txt` | Accepts | `I32(2)` | `Result: 2` | Passes | Yes |
| `valid_branch.mircap.txt` | Accepts | `I32(42)` | `Result: 42` | Passes | Yes |
| `valid_loop.mircap.txt` | Accepts | `I32(3)` | `Result: 3` | Passes | Yes |
| `valid_direct_call.mircap.txt` | Accepts | `I32(41)` | `Result: 41` | Passes | Yes |
| `valid_alloc_store_load_i32.mircap.txt` | Accepts | `I32(42)` | `Result: 42` | Passes | Yes |
| `valid_alloc_store_load_u32.mircap.txt` | Accepts | `U32(42)` | `Result: 42` | Passes | Yes |
| `valid_data_segment.mircap.txt` | Accepts | `I32(42)` | `Result: 42` | Passes | Yes |
| `valid_addr_add_two_cells.mircap.txt` | Accepts | `I32(42)` | `Result: 42` | Passes | Yes |
| `valid_memory_loop_sum.mircap.txt` | Accepts | `I32(28)` | `Result: 28` | Passes | Yes |
| `valid_sieve_32.mircap.txt` | Accepts | `I32(11)` | `Result: 11` | Passes | Yes |
| `valid_arithmetic_u32.mircap.txt` | Accepts | `U32(1)` | `Result: 1` | Passes | Yes |
| `valid_sieve_32_u32.mircap.txt` | Accepts | `U32(11)` | `Result: 11` | Passes | Yes |
| `valid_data_segment_load.mircap.txt` | Accepts | `U32(43)` | `Result: 43` | Passes | Yes |
| `valid_load_store_u8.mircap.txt` | Accepts | `U32(171)` | `Result: 171` | Passes | Yes |
| `valid_i64_ops.mircap.txt` | Accepts | `I32(42)` | `Result: 42` | Passes | Yes |
| `valid_float_constants.mircap.txt` | Accepts | `F32(1.5), F64(-0.25)` | N/A | N/A | Skipped |
| `valid_float_arithmetic.mircap.txt` | Accepts | `F32(-16.0), F64(-16.0)` | N/A | N/A | Skipped |

---

## 2. Execution Trap Fixtures

These fixtures are structurally and semantically valid under static analysis, but contain logic that deliberately triggers execution traps at runtime.

| Fixture Path | `mircap` Load / Validate | `mirsem` Execution Trap Kind | `mirc0` Transpiled Execution Trap | Strict C11 Compile | Differential Match |
|---|---|---|---|---|---|
| `trap_data_addr_dynamic_oob.mircap.txt` | Accepts | `OutOfBoundsLoad` (code 13) | `Trap: 13 OutOfBoundsLoad` | Passes | Yes |
| `trap_store_oob.mircap.txt` | Accepts | `OutOfBoundsStore` (code 14) | `Trap: 14 OutOfBoundsStore` | Passes | Yes |
| `trap_load_oob.mircap.txt` | Accepts | `OutOfBoundsLoad` (code 13) | `Trap: 13 OutOfBoundsLoad` | Passes | Yes |

---

## 3. Invalid (Validation Failure) Fixtures

These fixtures contain deliberate structural or type violations and must be rejected by `mircap` at load/validation time. They are not parsed into execution-ready `ModuleImage` representations, hence execution and transpilation do not apply (`N/A`).

| Fixture Path | `mircap` Validation Outcome | `mirsem` / `mirc0` Execution |
|---|---|---|
| `invalid_duplicate_function_id.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_missing_block.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_wrong_call_arity.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_return_type_mismatch.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_instruction_after_terminator.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_block_without_terminator.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_load_non_addr32.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_store_wrong_value_type.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_alloc_wrong_result_type.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_malformed_data_segment.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_addr_add_wrong_offset_type.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_addr_add_wrong_base_type.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_addr_add_addr32_offset.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_data_addr_static_oob.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_mixed_arithmetic.mircap.txt` | Rejects (ValidationError) | N/A |
| `invalid_addr32_normal_arithmetic.mircap.txt` | Rejects (ValidationError) | N/A |
