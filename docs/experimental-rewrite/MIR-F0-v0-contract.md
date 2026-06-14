# MIR-F0 v0 Language and Differential Testing Contract

This document serves as the formal specification and testing contract for the **MIR-F0 v0** experimental subset. It defines the types, instruction semantics, memory layout, static validation rules, execution trap conditions, and the differential verification contract between the `mirsem` oracle and the `mirc0` C11 backend.

## 1. Supported Types
MIR-F0 v0 supports the following primitive types:
- `void`
- `i32` (signed 32-bit integer)
- `u32` (unsigned 32-bit integer)
- `i64` (signed 64-bit integer)
- `addr32` (linear memory address pointer)
- `f32` (single-precision float)
- `f64` (double-precision float)

*Note: Booleans are represented using a `u32` integer convention (zero is false, non-zero is true).*

## 2. Immutability of ModuleImage
- The `ModuleImage` defined in `mircap` is strictly **immutable** once parsed and loaded.
- All dynamic execution states, register values, call stack frames, heap pointers, linear memory buffers, execution traces, and profile statistics remain completely separate from the `ModuleImage`.

## 3. Supported Opcodes
- **Constants & Copying**: `const_i32`, `const_u32`, `const_i64`, `const_f32`, `const_f64`, `copy`
- **Signed Arithmetic & Comparisons**: `add_i32`, `sub_i32`, `mul_i32`, `eq_i32`, `ne_i32`, `lt_i32`
- **Unsigned Arithmetic & Comparisons**: `add_u32`, `sub_u32`, `mul_u32`, `eq_u32`, `ne_u32`, `lt_u32`, `le_u32`, `gt_u32`, `ge_u32`
- **64-bit Integer Operations**: `add_i64`, `sub_i64`, `mul_i64`, `eq_i64`, `ne_i64`, `lt_i64`
- **Float Oracle Operations**: `add_f32`, `sub_f32`, `mul_f32`, `div_f32`, `neg_f32`, `add_f64`, `sub_f64`, `mul_f64`, `div_f64`, `neg_f64`
- **Memory Operations**: `alloc`, `load_i32`, `load_u32`, `load_i64`, `load_u8`, `store_i32`, `store_u32`, `store_i64`, `store_u8`, `addr_add`, `data_addr`
- **Control Flow & Execution**: `branch`, `branch_if`, `call`, `ret`, `trap`

Float support is partial. Float constants and arithmetic are part of validation
and `mirsem`; float comparisons, conversions, memory operations, C emission,
RV32 emission, and JIT FFI are still unsupported.

## 4. Execution Traps
When an execution trap is triggered, execution halts immediately. 

### Trap Identity Definition
A trap's identity is defined by a structured trap kind and its stderr line, not merely a non-zero process exit code. The standard format for a trap output line to stderr is:
`Trap: <code> <name>`

The mapping of exit codes and stderr names is defined as:
| Code | Trap Name | Description |
|---|---|---|
| 1 | `StackOverflow` | Call depth exceeds the execution profile limit |
| 2 | `FuelExhausted` | Instruction count limit reached |
| 3 | `ExplicitTrap` | Explicit execution of the `trap` instruction |
| 11 | `OutOfMemory` | Heap allocation fails due to insufficient memory |
| 12 | `HeapStackCollision` | Heap pointer and stack base collide |
| 13 | `OutOfBoundsLoad` | Load access address is outside valid linear memory or segment bounds |
| 14 | `OutOfBoundsStore` | Store access address is outside valid linear memory or segment bounds |
| 15 | `MisalignedLoad` | 4-byte load address is not 4-byte aligned |
| 16 | `MisalignedStore` | 4-byte store address or allocation alignment parameter is misaligned |
| 17 | `AddressOverflow` | Address calculation overflows the 32-bit address space |

## 5. Memory Model & Zero-Initialization
- **Linear Memory**: Bounded 32-bit linear memory space. `addr32` represents a byte offset from the start of linear memory.
- **Memory Initialization**: The entire linear memory space is zero-initialized to `0x00` at startup, except for the bytes loaded by static data segments.
- **Default Layout**:
  - Data segments loaded at static offsets near the start of memory.
  - Heap starts immediately following the data segments and grows upward.
  - Stack starts at the top of linear memory (`MEMORY_SIZE - STACK_SIZE`) and grows downward.
- **Alignment**: `load_i32`, `load_u32`, `store_i32`, and `store_u32` require their address to be a multiple of 4. `load_i64` and `store_i64` require 8-byte alignment. Byte operations (`load_u8`, `store_u8`) have no alignment restrictions.
- **Allocation**: `alloc(size, align)` increments the heap pointer, checking alignment and stack/heap collision. Alignment must be a power of two.

## 6. Data Segment Model & Mutability
- Static data segments are initialized in linear memory before any function executes.
- They have a declared static base offset, byte array, and a zero-filled trailing region.
- The total length of a data segment is `segment_len = bytes.len() + zero_fill`.
- **Runtime Mutability**: Data segments reside in linear memory, which is completely mutable. Once initialized, the memory occupied by data segments can be read or written to using load and store instructions; they are not read-only.

### `data_addr` Semantics
- `data_addr(data_segment_symbol, offset)` returns `addr32` calculated as `ds.offset + offset`.
- **Static Boundary Checking**: If the `offset` is a static immediate constant, validation enforces that `offset <= segment_len`. A static offset exceeding `segment_len` (`offset > segment_len`) fails validation.
- **Dynamic Boundary Checking**: If the `offset` is a dynamic value, validation allows it, but runtime execution checks that `offset <= segment_len`. A dynamic offset exceeding `segment_len` triggers an `OutOfBoundsLoad` trap (code 13).
- **The Boundary Rule**: `offset == segment_len` is explicitly **allowed**. This enables standard pointer-arithmetic idioms (such as pointing to "one past the end" of a buffer to represent end boundaries). However, performing a read or write operation at this boundary address will trigger an execution trap.

## 7. Instruction Semantics

### Register Zero-Initialization
- To avoid uninitialized reads and undefined C behavior, all virtual registers and local variables are initialized to zero (`0` or `0u`) in both `mirsem` and `mirc0` at the entry of each function.

### Byte Load/Store Operations
- `load_u8` reads one byte from linear memory and zero-extends it to `u32`.
- `store_u8` writes one byte to linear memory, masking the input value to its lowest 8 bits (`value & 0xFF`).

### Address Arithmetic (`addr_add`)
- `addr_add(base: addr32, offset: u32) -> addr32` performs explicit address calculations. 
- It adds the unsigned offset to the base address.
- If the addition overflows the 32-bit address space (`base + offset > 0xFFFFFFFF`), it triggers an `AddressOverflow` execution trap (code 17).

### Arithmetic
- Signed `i32` arithmetic avoids undefined behavior by performing wrapping arithmetic as unsigned operations and casting back.
- Signed `i64` arithmetic uses wrapping semantics and is lowered through supported interpreter, C, upstream-diff, and RV32I paths.
- Unsigned `u32` arithmetic operates with wrapping semantics.
- Float arithmetic uses Rust `f32`/`f64` behavior in `mirsem` and stores results as exact IEEE-754 bit patterns. The current float contract deliberately excludes exception flags and backend emission.

### Branch Semantics
- `branch_if(cond, true_target, false_target)` branches depending on whether `cond` is non-zero (true) or zero (false). Both targets must be explicit; fallthrough is not assumed.

### Functions & Calls
- Functions support 0 or 1 result. 
- Direct calls must match callee signature parameter and result counts.

## 8. Validation Rules
The static validator (`mircap`) checks structural correctness, unique IDs, terminator existence, parameter types, matching value_types counts, and strict type constraints. Unsupported features like indirect calls, reserved float comparison/conversion opcodes, and legacy upstream-only float markers are rejected.

## 9. Roles in Differential Testing
- **`mirsem` Oracle**: Executes the validated `ModuleImage` directly as the semantic oracle.
- **`mirc0` C11 Backend**: Transpiles the `ModuleImage` to a single C11 source file with an embedded runtime.
  - **Typed Result Printing**: Emits the result to stdout prefixed by `Result: ` based on the return type of the entry function:
    - `void` returns print `Result: void`
    - `i32` returns print `Result: <signed decimal>`
    - `u32` / `addr32` returns print `Result: <unsigned decimal>`
    - `i64` returns print `Result: i64 <signed decimal>`
- **Differential Testing**: Valid integer/address/memory images are compiled using `cc -std=c11 -Wall -Wextra -Werror -O0` and run. Their result (value or exact trap identity) must match `mirsem`'s result exactly. Float fixtures currently run in `mirsem` only and are skipped by `diff-all` until float C emission exists.
