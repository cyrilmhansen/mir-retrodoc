# JIT API And Generation Modes

## Overview

The public generator API is declared in `mir-gen.h:19-27`:

- `MIR_gen_init`
- `MIR_gen_set_debug_file`
- `MIR_gen_set_debug_level`
- `MIR_gen_set_optimize_level`
- `MIR_gen`
- `MIR_set_gen_interface`
- `MIR_set_lazy_gen_interface`
- `MIR_set_lazy_bb_gen_interface`
- `MIR_gen_finish`

Observed user-facing execution modes:

- Explicit generation: caller invokes `MIR_gen(ctx, func_item)` directly.
- Eager generated interface: `MIR_link(ctx, MIR_set_gen_interface, resolver)` compiles each linked MIR function.
- Lazy function generation: `MIR_link(ctx, MIR_set_lazy_gen_interface, resolver)` installs a wrapper that compiles the whole function on first call.
- Lazy basic-block generation: `MIR_link(ctx, MIR_set_lazy_bb_gen_interface, resolver)` installs a wrapper that prepares the function on first call and then generates basic-block versions on first execution.

The MIR manual describes these three link-time interfaces in `MIR.md:699-715`. `MIR_link` calls the selected `set_interface` callback once for each function in newly loaded modules and once more with `func_item == NULL` as an interface-finish signal (`mir.c:2061-2072`).

Before using generator interfaces, callers should call `MIR_gen_init(ctx)`. The manual states this requirement in `MIR.md:746-751`, and the setter functions explicitly terminate the process if called before `MIR_gen_init` (`mir-gen.c:9509-9538`, `mir-gen.c:9707-9713`).

## Explicit Generation With `MIR_gen`

`MIR_gen(ctx, func_item)` is a thin wrapper around `generate_func_code(ctx, func_item, TRUE)` (`mir-gen.c:9505-9506`).

Observed behavior:

- It expects a function item whose `item_type` is `MIR_func_item` and whose temporary `item->data` field is `NULL` (`mir-gen.c:9285`).
- It compiles one complete MIR function when `machine_code_p == TRUE`.
- If `func_item->u.func->machine_code` is already non-`NULL`, it redirects the function thunk to the existing `call_addr` and returns the function thunk address (`mir-gen.c:9285-9293`).
- On a new compile it publishes native code, stores the raw generated address in `func_item->u.func->machine_code`, stores the call target in `func_item->u.func->call_addr`, redirects the function thunk `func_item->addr`, restores the original MIR instruction list, and returns `func_item->addr` (`mir-gen.c:9474-9502`).

Important distinction: `MIR_gen` returns the callable function thunk address, not necessarily the same pointer as the raw generated machine-code body. The raw body is stored in `func_item->u.func->machine_code`; `call_addr` may differ when call tracing is enabled (`mir-gen.c:9474-9485`).

## Eager Generated Interface

`MIR_set_gen_interface(ctx, func_item)` is intended for use as the `set_interface` callback passed to `MIR_link`.

Observed behavior:

- For each non-`NULL` function item, it calls `MIR_gen(ctx, func_item)` (`mir-gen.c:9755-9760`).
- On the final `func_item == NULL` call, it calls `target_change_to_direct_calls(ctx)` (`mir-gen.c:9755-9758`).

The manual describes this mode as generating machine code for all loaded MIR functions and making calls from MIR code execute machine code (`MIR.md:705-708`).

Classification:

- Whole-function native generation: `required for native host execution`.
- Final direct-call rewrite: `backend-specific engineering detail` and `optimization convenience`.
- MIR function semantics: `essential to MIR semantics`, but not inherently tied to native generation.

## Lazy Function Generation

`MIR_set_lazy_gen_interface(ctx, func_item)` installs a generated wrapper rather than compiling the function immediately.

Observed behavior:

- It ignores the final `func_item == NULL` callback (`mir-gen.c:9779-9784`).
- For each function item, it creates a wrapper with `_MIR_get_wrapper(ctx, func_item, generate_func_and_redirect_to_func_code)` and redirects the function thunk to that wrapper (`mir-gen.c:9779-9785`).
- On first call, the wrapper invokes `generate_func_and_redirect_to_func_code`, which calls `generate_func_and_redirect(ctx, func_item, TRUE)` and returns `func_item->u.func->machine_code` (`mir-gen.c:9763-9776`).
- `generate_func_and_redirect(..., TRUE)` calls `generate_func_code(..., TRUE)`, so the first call compiles the complete function (`mir-gen.c:9763-9766`).

The manual describes this as generating function machine code on the first function call (`MIR.md:709-712`).

The wrapper/thunk mechanism is target-specific. For example, x86-64 wrappers save caller-visible registers, call a C hook, restore registers, and jump to the returned target (`mir-x86_64.c:918-971`). RISC-V64 has analogous target code in `mir-riscv64.c:1124-1165`.

Classification:

- Lazy policy: `optimization convenience`.
- Wrapper/thunk code: `required for native host execution` and `backend-specific engineering detail`.
- C hook calling convention inside wrapper: `required only for C ABI compatibility`.

## Lazy Basic-Block Generation

`MIR_set_lazy_bb_gen_interface(ctx, func_item)` installs a wrapper that prepares lazy block generation on the first function call.

Observed behavior:

- It ignores the final `func_item == NULL` callback (`mir-gen.c:10007-10012`).
- For each function, it creates a wrapper with `_MIR_get_wrapper(ctx, func_item, generate_func_and_redirect_to_bb_gen)` and redirects the function thunk to that wrapper (`mir-gen.c:10007-10012`).
- On first call, `generate_func_and_redirect_to_bb_gen` calls `generate_func_and_redirect(ctx, func_item, FALSE)` and returns `func_item->addr` (`mir-gen.c:10001-10004`).
- `generate_func_and_redirect(..., FALSE)` calls `generate_func_code(..., FALSE)`, then builds `bb_stub` records with `create_bb_stubs`, creates the entry block version, and redirects the function thunk to the entry block thunk/address (`mir-gen.c:9763-9771`).
- In `generate_func_code(..., FALSE)`, the generator runs the preparatory pipeline but skips whole-function `target_translate`, code publication, thunk redirection, instruction restoration, and `machine_code` assignment (`mir-gen.c:9474-9502`).

The manual describes this mode as generating machine code for function basic blocks only on their first execution (`MIR.md:713-715`).

This mode is optional relative to MIR semantics. It relies on property instructions such as `MIR_PRSET`, `MIR_PRBEQ`, and `MIR_PRBNE`, which the manual describes as producing no ordinary machine instruction but enabling specialized code under lazy basic-block versioning (`MIR.md:543-548`).

## Relationship With Linking

`MIR_link` performs more than interface installation. It also simplifies functions, marks functions needing inline processing, resolves imports/exports/forwards, evaluates expression data through the interpreter, and then invokes `set_interface` on linked function items (`mir.c:1969-2072`).

Observed API sequence for generated execution:

1. Initialize and build or read MIR: `MIR_init`, module construction or scanning, `MIR_finish_module`, `MIR_load_module`.
2. Initialize the generator: `MIR_gen_init`.
3. Optionally configure generator debug or optimization level.
4. Link with a generator interface: `MIR_link(ctx, MIR_set_gen_interface | MIR_set_lazy_gen_interface | MIR_set_lazy_bb_gen_interface, resolver)`.
5. Call `func_item->addr` through an appropriate C function pointer type.
6. Finish generator state with `MIR_gen_finish` before context teardown if generator state is no longer needed.
7. Finish the context with `MIR_finish`.

This sequence is visible in the C2MIR driver: generator initialization and options occur before `MIR_link`, the selected interface is passed to `MIR_link`, then `main_func->addr` is called as a C function pointer (`c2mir/c2mir-driver.c:907-928`).

## Generated Call Addresses

Function items loaded by `MIR_load_module` receive a target thunk if `item->addr == NULL`; the thunk is initially redirected to an undefined interface (`mir.c:1915-1935`).

The JIT modes reuse that thunk:

- Eager generation redirects `func_item->addr` to `func_item->u.func->call_addr` after code publication (`mir-gen.c:9474-9485`).
- Lazy function generation redirects `func_item->addr` to a wrapper first, then the first-call hook redirects it to the generated function body (`mir-gen.c:9774-9785`).
- Lazy BB generation redirects `func_item->addr` to a wrapper first, then to the entry basic-block version thunk/address after preparatory generation (`mir-gen.c:9763-9771`, `mir-gen.c:10001-10012`).

Generated code is published through `_MIR_publish_code`, which allocates from context-owned code holders, writes bytes through `_MIR_set_code`, marks pages executable, flushes the instruction cache, and returns the executable address (`mir.c:4361-4428`).

## Limitations And Trade-Offs

- `MIR_gen` is whole-function generation. It is not a trace compiler and does not compile smaller regions in explicit/eager/lazy-function mode.
- Lazy function generation reduces upfront link-time cost but still pays full function compile cost on first call.
- Lazy BB generation reduces first-execution granularity, but it depends on mutable thunks, target wrappers, and a specialized basic-block version mechanism.
- `MIR_set_gen_interface` performs a target-specific final direct-call rewrite, but the lazy interfaces ignore the `func_item == NULL` finalization callback.
- The generator API assumes native executable memory and target-specific wrappers/thunks. That is outside pure MIR IR semantics.
- The current generator target selection explicitly includes only RISC-V64 for RISC-V and rejects non-64-bit RISC-V targets (`mir-gen.c:321-329`).

## Relevance To RISC-V32 / Fantasy Computer Extraction

- MIR context/module/function/item representation: `essential to MIR semantics`.
- Whole-function JIT API shape: `required for native host execution` in current MIR, but removable for an interpreter-only or bytecode fantasy subset.
- Lazy function wrappers: `optimization convenience` plus `backend-specific engineering detail`.
- Lazy BB wrappers and thunks: `optimization convenience`; likely removable for a fantasy subset unless block-version specialization is a goal.
- C-callable function addresses and wrapper hooks: `required only for C ABI compatibility`.
- Executable memory publication and instruction-cache flushing: `required for native host execution`.

For RISC-V32, the API names could remain stable, but a backend would need new 32-bit thunk/wrapper code, a 32-bit calling convention lowering path, pointer-size-sensitive relocation/code emission, and appropriate instruction-cache synchronization. The existing RISC-V path is not a RISC-V32 implementation.

## Open Questions

- Does `MIR_gen` have defined behavior if called before `MIR_gen_init`, or is that an unchecked invalid use?
- Are users expected to call `MIR_gen_finish` before `MIR_finish`, or is generator teardown intentionally separate only when reusing a context?
- What exact transformations does `target_change_to_direct_calls` perform on each backend?
- Is lazy BB generation used outside C2MIR driver modes and tests?
- Are generated function thunks safe to call concurrently while lazy generation or thunk patching is in progress?
