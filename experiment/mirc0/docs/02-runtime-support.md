# Runtime Support

`mirc0` embeds a minimal, portable C runtime at the top of the emitted code.

## Aligned Linear Memory
The runtime declares a static array aligned to 4 bytes:
```c
static uint32_t g_memory_aligned[MEMORY_SIZE / 4];
#define g_memory ((uint8_t*)g_memory_aligned)
```

By default, `MEMORY_SIZE` is 1024 KiB, and `STACK_SIZE` is 64 KiB. These can be overridden at compile-time (e.g. `-DMEMORY_SIZE=128`).

## Runtime Helpers
- `mir_alloc(size, align)`: Aligns `g_heap_ptr`, checks heap/stack boundaries, and returns address.
- `mir_load_i32(addr)` / `mir_load_u32(addr)`: Asserts 4-byte alignment, performs bounds checks, and returns little-endian value.
- `mir_store_i32(addr, val)` / `mir_store_u32(addr, val)`: Asserts 4-byte alignment, performs bounds checks, and writes little-endian value.
- `mir_addr_add(base, offset)`: Standard bounds-checked address addition.
- `mir_trap(code)`: Prints `Trap: <code> <name>` to `stderr` and calls `exit(code)`.

## Trap Codes
1: StackOverflow
2: FuelExhausted
3: ExplicitTrap
11: OutOfMemory
12: HeapStackCollision
13: OutOfBoundsLoad
14: OutOfBoundsStore
15: MisalignedLoad
16: MisalignedStore
17: AddressOverflow
