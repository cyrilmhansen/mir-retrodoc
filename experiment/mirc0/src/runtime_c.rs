pub const RUNTIME_HEADER: &str = r#"#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>

#ifndef MEMORY_SIZE
#define MEMORY_SIZE (1024 * 1024)
#endif

#ifndef STACK_SIZE
#define STACK_SIZE (64 * 1024)
#endif

static uint32_t g_memory_aligned[MEMORY_SIZE / 4];
#define g_memory ((uint8_t*)g_memory_aligned)
static uint32_t g_heap_ptr = 0;
static uint32_t g_stack_base = MEMORY_SIZE - STACK_SIZE;

void mir_trap(int code) {
    const char *name = "Unknown";
    switch (code) {
        case 1: name = "StackOverflow"; break;
        case 2: name = "FuelExhausted"; break;
        case 3: name = "ExplicitTrap"; break;
        case 11: name = "OutOfMemory"; break;
        case 12: name = "HeapStackCollision"; break;
        case 13: name = "OutOfBoundsLoad"; break;
        case 14: name = "OutOfBoundsStore"; break;
        case 15: name = "MisalignedLoad"; break;
        case 16: name = "MisalignedStore"; break;
        case 17: name = "AddressOverflow"; break;
    }
    fprintf(stderr, "Trap: %d %s\n", code, name);
    exit(code);
}

uint32_t mir_alloc(uint32_t size, uint32_t align) {
    if (align == 0 || (align & (align - 1)) != 0) {
        mir_trap(16); // MisalignedStore / invalid alignment
    }
    uint32_t mask = align - 1;
    if (g_heap_ptr > 0xFFFFFFFFu - mask) {
        mir_trap(11); // OutOfMemory
    }
    uint32_t aligned = (g_heap_ptr + mask) & ~mask;
    if (size > 0xFFFFFFFFu - aligned) {
        mir_trap(11); // OutOfMemory
    }
    uint32_t end = aligned + size;
    if (end > g_stack_base) {
        mir_trap(12); // HeapStackCollision
    }
    g_heap_ptr = end;
    return aligned;
}

int32_t mir_load_i32(uint32_t addr) {
    if (addr % 4 != 0) {
        mir_trap(15); // MisalignedLoad
    }
    if (addr > MEMORY_SIZE - 4) {
        mir_trap(13); // OutOfBoundsLoad
    }
    uint32_t val = (uint32_t)g_memory[addr] |
                   ((uint32_t)g_memory[addr + 1] << 8) |
                   ((uint32_t)g_memory[addr + 2] << 16) |
                   ((uint32_t)g_memory[addr + 3] << 24);
    return (int32_t)val;
}

uint32_t mir_load_u32(uint32_t addr) {
    if (addr % 4 != 0) {
        mir_trap(15); // MisalignedLoad
    }
    if (addr > MEMORY_SIZE - 4) {
        mir_trap(13); // OutOfBoundsLoad
    }
    uint32_t val = (uint32_t)g_memory[addr] |
                   ((uint32_t)g_memory[addr + 1] << 8) |
                   ((uint32_t)g_memory[addr + 2] << 16) |
                   ((uint32_t)g_memory[addr + 3] << 24);
    return val;
}

void mir_store_i32(uint32_t addr, int32_t value) {
    if (addr % 4 != 0) {
        mir_trap(16); // MisalignedStore
    }
    if (addr > MEMORY_SIZE - 4) {
        mir_trap(14); // OutOfBoundsStore
    }
    uint32_t val = (uint32_t)value;
    g_memory[addr] = val & 0xFF;
    g_memory[addr + 1] = (val >> 8) & 0xFF;
    g_memory[addr + 2] = (val >> 16) & 0xFF;
    g_memory[addr + 3] = (val >> 24) & 0xFF;
}

void mir_store_u32(uint32_t addr, uint32_t value) {
    if (addr % 4 != 0) {
        mir_trap(16); // MisalignedStore
    }
    if (addr > MEMORY_SIZE - 4) {
        mir_trap(14); // OutOfBoundsStore
    }
    g_memory[addr] = value & 0xFF;
    g_memory[addr + 1] = (value >> 8) & 0xFF;
    g_memory[addr + 2] = (value >> 16) & 0xFF;
    g_memory[addr + 3] = (value >> 24) & 0xFF;
}

uint32_t mir_addr_add(uint32_t base, uint32_t offset) {
    if (base > 0xFFFFFFFFu - offset) {
        mir_trap(17); // AddressOverflow
    }
    return base + offset;
}

uint32_t mir_data_addr(uint32_t base, uint32_t offset, uint32_t len) {
    if (offset > len) {
        mir_trap(13); // OutOfBoundsLoad / dynamic out-of-range offset
    }
    if (base > 0xFFFFFFFFu - offset) {
        mir_trap(17); // AddressOverflow
    }
    return base + offset;
}

uint32_t mir_load_u8(uint32_t addr) {
    if (addr > MEMORY_SIZE - 1) {
        mir_trap(13); // OutOfBoundsLoad
    }
    return (uint32_t)g_memory[addr];
}

void mir_store_u8(uint32_t addr, uint32_t value) {
    if (addr > MEMORY_SIZE - 1) {
        mir_trap(14); // OutOfBoundsStore
    }
    g_memory[addr] = (uint8_t)(value & 0xFF);
}
"#;
