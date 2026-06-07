# Memory Profile

`mirsem` owns execution resources. `mircap` only describes the immutable module
image and validates static structure.

## Default Profile

- linear memory size: 1024 KiB;
- stack reservation: 64 KiB at the top of memory;
- heap allocator: bump allocator;
- endianness: little-endian;
- invalid access: execution trap;
- host pointers: forbidden.

## Linear Memory

`addr32` values are offsets into bounded linear memory, not host pointers.

Data segments from `mircap` are initialized at their declared offsets before
execution. The heap pointer starts after the initialized/zero-filled data range.
Heap grows upward. Stack is reserved at the top of memory and grows downward in
future runtime modules.

There is not yet a MIR-F0 `data_addr`/`global_addr` instruction, so executable
fixtures cannot directly take the address of an initialized data segment.

`alloc(size, align)` uses the bump allocator. There is no free, realloc, or GC.

`addr_add(addr32, u32) -> addr32` is the only address arithmetic currently
supported. It does not permit implicit casts between addresses and integers.

## Validation vs Execution

Malformed memory instructions are `mircap` validation errors. Out-of-bounds
access, heap/stack collision, out-of-memory, and misalignment are `mirsem`
execution traps.
