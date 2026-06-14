# MIR-F0 Subset

MIR-F0 is a minimal experimental subset, not full MIR.

MIR-F0 compatibility means compatibility with this explicitly documented subset
only. Unsupported upstream MIR features must not be silently accepted.

## Supported Types

- `void`
- `i32`
- `u32`
- `i64`
- `addr32` as a reserved address type
- `f32`
- `f64`

Boolean values use an integer convention. They are not currently a separate
runtime type.

## Unsupported Types

- legacy upstream `float` marker type: `reject-at-load-time`
- `long double`: `out-of-scope`
- C aggregates: `out-of-scope`
- varargs: `out-of-scope`
- host C ABI types: `out-of-scope`

## Supported Opcodes

- `const_i32`
- `const_u32`
- `const_i64`
- `const_f32`
- `const_f64`
- `copy`
- `add_i32`
- `sub_i32`
- `mul_i32`
- `eq_i32`
- `ne_i32`
- `lt_i32`
- `add_u32`
- `sub_u32`
- `mul_u32`
- `eq_u32`
- `ne_u32`
- `lt_u32`
- `le_u32`
- `gt_u32`
- `ge_u32`
- `add_i64`
- `sub_i64`
- `mul_i64`
- `eq_i64`
- `ne_i64`
- `lt_i64`
- `add_f32`
- `sub_f32`
- `mul_f32`
- `div_f32`
- `neg_f32`
- `add_f64`
- `sub_f64`
- `mul_f64`
- `div_f64`
- `neg_f64`
- `branch`
- `branch_if`
- `call`
- `ret`
- `trap`
- `alloc`
- `load_i32`
- `load_u32`
- `load_i64`
- `load_u8`
- `store_i32`
- `store_u32`
- `store_i64`
- `store_u8`
- `addr_add`
- `data_addr`

## Memory Opcode Semantics

`alloc(size, align) -> addr32` allocates from linear memory. `size` and `align`
are `u32` immediates or integer values. `align` must be non-zero and a power of
two at execution time. Allocation failure is an execution trap, not a
module-image validation error.

`load_i32(addr32) -> i32` and `load_u32(addr32) -> u32` read four bytes from
linear memory using little-endian layout. `load_i64(addr32) -> i64` reads eight
bytes from linear memory using little-endian layout.

`load_u8(addr32) -> u32` reads a single byte from linear memory and zero-extends
it to a `u32`.

`store_i32(addr32, i32) -> void` and `store_u32(addr32, u32) -> void` write four
bytes using little-endian layout. `store_i64(addr32, i64) -> void` writes eight
bytes. `store_u32` is separate from `store_i32` to avoid ambiguity in static
validation.

`store_u8(addr32, u32) -> void` writes a single byte to linear memory, masking
the input value to its lowest 8 bits (`value & 0xFF`).

`addr_add(addr32, u32) -> addr32` performs explicit address arithmetic. MIR-F0
does not allow implicit casts between `addr32` and `u32`.

`data_addr(data_segment_symbol, offset: u32) -> addr32` calculates the linear
memory address of a static data segment. The base of the address is the segment's
static loading offset (`ds.offset`). The valid range of offsets is `0 <= offset <= segment_len`
where `segment_len` is the total size of the data segment (bytes length + zero fill).
If a static offset is an immediate constant and exceeds `segment_len`, it fails
static validation. If it is a dynamic value and exceeds `segment_len` at runtime,
it triggers an execution trap (`OutOfBoundsLoad`).

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
- lazy basic-block versioning: `out-of-scope`
- float comparisons, conversions, memory operations, C emission, RV32 emission,
  and JIT FFI bridge: deferred
- exception handling: `out-of-scope`
- SSA phi nodes: `out-of-scope`
- host C ABI constructs: `out-of-scope`

## Memory Model

MIR-F0 uses bounded 32-bit linear memory. `addr32` values are offsets into that
memory, not host pointers. The default executor profile is expected to use 1024
KiB of linear memory with a 64 KiB stack reservation at the top. Globals/data
segments live at the beginning of memory, heap grows upward, and stack grows
downward.

Strict alignment and invalid accesses are execution-time traps. MIR-F0 has no
free, realloc, GC, host pointer exposure, host C ABI memory, or dynamic linking
to external C symbols.
