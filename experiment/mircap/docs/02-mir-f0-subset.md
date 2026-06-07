# MIR-F0 Subset

MIR-F0 is a minimal experimental subset, not full MIR.

MIR-F0 compatibility means compatibility with this explicitly documented subset
only. Unsupported upstream MIR features must not be silently accepted.

## Supported Types

- `void`
- `i32`
- `u32`
- `addr32` as a reserved address type

Boolean values use an integer convention. They are not currently a separate
runtime type.

## Unsupported Types

- `i64`: `reject-at-load-time`
- floating point: `out-of-scope`
- `long double`: `out-of-scope`
- C aggregates: `out-of-scope`
- varargs: `out-of-scope`
- host C ABI types: `out-of-scope`

## Supported Opcodes

- `const_i32`
- `const_u32`
- `copy`
- `add_i32`
- `sub_i32`
- `mul_i32`
- `eq_i32`
- `ne_i32`
- `lt_i32`
- `branch`
- `branch_if`
- `call`
- `ret`
- `trap`
- `alloc`
- `load_i32`
- `load_u32`
- `store_i32`
- `store_u32`
- `addr_add`

## Memory Opcode Semantics

`alloc(size, align) -> addr32` allocates from linear memory. `size` and `align`
are `u32` immediates or integer values. `align` must be non-zero and a power of
two at execution time. Allocation failure is an execution trap, not a
module-image validation error.

`load_i32(addr32) -> i32` and `load_u32(addr32) -> u32` read four bytes from
linear memory using little-endian layout.

`store_i32(addr32, i32) -> void` and `store_u32(addr32, u32) -> void` write four
bytes using little-endian layout. `store_u32` is separate from `store_i32` to
avoid ambiguity in static validation.

`addr_add(addr32, u32) -> addr32` performs explicit address arithmetic. MIR-F0
does not allow implicit casts between `addr32` and `u32`.

Out-of-bounds access and misalignment are execution traps owned by `mirsem` or
another executor.

## Control Flow

`branch_if(cond, true_target, false_target)` has two explicit targets.
`cond` is a `u32` boolean convention: non-zero selects `true_target`, zero
selects `false_target`.

MIR-F0 block order does not define conditional fallthrough semantics. Control
flow must be explicit.

## Unsupported Opcodes

- indirect calls: `reject-at-load-time`
- i64 memory operations: `reject-at-load-time`
- lazy basic-block versioning: `out-of-scope`
- floating-point operations: `out-of-scope`
- exception handling: `out-of-scope`
- SSA phi nodes: `out-of-scope`
- host C ABI constructs: `out-of-scope`

`load_u8` and `store_u8` are deferred. They are likely useful for strings or a
BASIC-like environment, but they are not part of the first memory subset.

## Memory Model

MIR-F0 uses bounded 32-bit linear memory. `addr32` values are offsets into that
memory, not host pointers. The default executor profile is expected to use 1024
KiB of linear memory with a 64 KiB stack reservation at the top. Globals/data
segments live at the beginning of memory, heap grows upward, and stack grows
downward.

Strict alignment and invalid accesses are execution-time traps. MIR-F0 has no
free, realloc, GC, host pointer exposure, host C ABI memory, or dynamic linking
to external C symbols.
