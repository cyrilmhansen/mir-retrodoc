# MIR-F0 Subset

MIR-F0 is a minimal MIR-inspired subset for the first experiment.

## Supported

- modules;
- functions;
- stable IDs;
- explicit basic blocks;
- instruction lists;
- `void`, `i32`, `u32`, `i64`, `addr32`, `f32`, and `f64`;
- integer and float constants;
- copy;
- `i32` add/sub/mul;
- `i64` add/sub/mul;
- `f32` and `f64` add/sub/mul/div/neg in `mirsem`;
- `i32` comparisons;
- `i64` comparisons;
- unconditional branch;
- conditional branch;
- direct call;
- return;
- trap placeholder.
- minimal memory operations:
  - `alloc`;
  - `load_i32`;
  - `load_u32`;
  - `load_i64`;
  - `load_u8`;
  - `store_i32`;
  - `store_u32`;
  - `store_i64`;
  - `store_u8`;
  - `addr_add`.

## Deferred Or Unsupported

- float comparisons: deferred;
- integer/float conversions: deferred;
- float memory operations: deferred;
- float C/RV32/JIT emission: deferred;
- `long double`: `out-of-scope`;
- C aggregates: `out-of-scope`;
- varargs: `out-of-scope`;
- external C symbols: `out-of-scope`;
- indirect calls: `reject-at-load-time`;
- lazy BBV: `out-of-scope`;
- function redefinition: `out-of-scope`;
- runtime code replacement and deoptimization: `out-of-scope`.

MIR-F0 memory is bounded 32-bit linear memory. `addr32` is an offset into that
memory, not a host pointer. Runtime bounds, alignment, allocation failure, and
heap/stack collision are executor traps, not immutable-image state.
`addr_add(addr32, u32) -> addr32` is explicit; MIR-F0 does not allow implicit
casts between `addr32` and `u32`.

`branch_if(cond, true_target, false_target)` has two explicit targets. Block
order does not define conditional fallthrough semantics.

## Warning

A program accepted by upstream MIR is not necessarily valid MIR-F0.
