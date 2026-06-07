# API Lifecycle

## Overview

The public lifecycle starts with a `MIR_context_t`, creates one or more modules, optionally loads modules and external symbols, links imports/exports, and installs an execution interface such as interpreter or JIT.

Observed entry points are declared in `mir.h`: `MIR_init`, `MIR_init2`, `_MIR_init`, `MIR_finish`, `MIR_new_module`, `MIR_finish_module`, `MIR_load_module`, `MIR_load_external`, and `MIR_link` (`mir.h:474-492`, `mir.h:494-535`, `mir.h:621-624`).

The context owns most long-lived MIR state. Modules and items are stored in context-managed lists and hash tables. Generated code is also tied to the context-level machine-code allocator and released during `MIR_finish`.

## Context Lifecycle

`MIR_init` is an inline wrapper over `MIR_init2(NULL, NULL)`. `MIR_init2` checks `MIR_API_VERSION` against `_MIR_get_api_version` and then calls `_MIR_init` (`mir.h:474-489`).

`_MIR_init` accepts optional heap and executable-code allocators. If either is `NULL`, it installs the default allocator and default code allocator (`mir.c:733-739`). It then allocates `struct MIR_context`, initializes context sub-pointers to `NULL`, stores `alloc` and `code_alloc`, sets default error handling, clears `curr_module`, `curr_func`, and `curr_label_num`, and initializes string/alias tables, instruction descriptions, all-module list, simplifier, temporary buffers, optional binary IO, optional scanner, the modules-to-link vector, temporary operands, the environment module, item table, code cache, target hard-register names, and interpreter state (`mir.c:741-790`).

Observed fields in `struct MIR_context` (`mir.c:31-60`):

- IR ownership/state: `alloc`, `insn_nops`, `unspec_protos`, `temp_string`, `temp_data`, `used_label_p`, `module_item_tab`, `environment_module`, `curr_module`, `curr_func`, `curr_label_num`, `all_modules`, `modules_to_link`, `temp_ops`, `string_ctx`, `reg_ctx`, `alias_ctx`, `simplify_ctx`.
- Interpreter state: `interp_ctx`, `setjmp_addr`.
- JIT/code generation state: `gen_ctx`, `hard_reg_ctx`, `wrapper_end_addr`.
- Executable memory/code cache: `code_alloc`, `machine_code_ctx`.
- Frontend/IO/scanner state: `c2mir_ctx`, `io_ctx`, `scan_ctx`.

`MIR_finish` tears down the interpreter first, removes all modules and the environment module, destroys the item table and link/temp vectors, destroys scanner and binary IO state when enabled, destroys temporary buffers, unspec prototypes, string/alias tables, simplifier state, instruction arity table, code cache, and hard-register state, then checks that no current function or module remains open and frees the context (`mir.c:888-924`).

Important ordering fact: `MIR_finish` calls `remove_all_modules` before checking `curr_func`/`curr_module` (`mir.c:889-920`). If a caller finishes with an unclosed function or module, the code will still attempt module teardown before reporting the finish error. Treat unclosed current objects as invalid use.

## Module Lifecycle

`MIR_new_module` requires no currently open module. It allocates a `struct MIR_module`, initializes it, appends it to `all_modules`, and sets it as `curr_module` (`mir.c:927-936`). Several modules can coexist in one context because every finished module remains in `all_modules` and can be retrieved with `MIR_get_module_list` (`mir.h:494-495`, `mir.h:420-425`).

`MIR_finish_module` only checks that a module is open and then sets `curr_module = NULL` (`mir.c:1767-1772`). It does not load the module, allocate data sections, resolve imports, install call interfaces, or generate code.

Module items are created against `curr_module`. `create_item` rejects item creation outside a current module, initializes item metadata, and leaves `addr = NULL` until loading/linking or external setup (`mir.c:1073-1089`). Named items are added through `add_item`, which manages replacement/forward/export/import cases and rejects repeated concrete definitions (`mir.c:1015-1070`).

## Loading And Linking

`MIR_load_module` walks module items. For bss/data/ref/lref/expr-data items it allocates or lays out data with `load_bss_data_section`; for function items it creates a target thunk if `item->addr == NULL`, then redirects the thunk to `undefined_interface` (`mir.c:1915-1935`). Exported concrete items are placed in the context's environment/global item table through `setup_global`; repeated function definitions are rejected unless redefinition is permitted, with an observed macOS `__darwin` exception (`mir.c:1936-1950`). If the module contains label-reference data, `link_module_lrefs` verifies label/function relationships and links `MIR_lref_data_t` nodes into `MIR_func_t.first_lref` (`mir.c:1840-1913`, `mir.c:1952`). Finally, the module is pushed onto `modules_to_link` (`mir.c:1953`).

`MIR_link` processes every module in `modules_to_link`. It initializes simplification state, simplifies every function with `simplify_func(ctx, item, TRUE)`, marks functions needing inline processing by setting `item->data`, resolves imports through the environment table or `import_resolver`, and resolves export/forward items to local definitions (`mir.c:1969-2025`). It then processes inlines for marked functions, initializes ref-data and expr-data payloads, and if a `set_interface` callback is supplied, pops every module from `modules_to_link`, calls `finish_func_interpretation` for each function, invokes `set_interface(ctx, item)`, and finally calls `set_interface(ctx, NULL)` as an interface-finish signal (`mir.c:2025-2072`).

For JIT execution, the usual `set_interface` is `MIR_set_gen_interface` or `MIR_set_lazy_gen_interface`. `MIR_set_gen_interface` calls `MIR_gen` for each function and, on the final `NULL` callback, calls `target_change_to_direct_calls` (`mir-gen.c:9755-9761`). `MIR_gen` calls `generate_func_code(ctx, func_item, TRUE)`, which duplicates original instructions, builds CFG, runs optimization/lowering/register allocation, asks the target to translate, publishes code with `_MIR_publish_code`, rebases target references, redirects the function thunk to the generated `call_addr`, restores original MIR instructions, stores `machine_code`, and returns the thunk address (`mir-gen.c:9277-9506`).

## External Symbols

`MIR_load_external(ctx, name, addr)` records special `setjmp`/`_setjmp` addresses in `ctx->setjmp_addr` and otherwise calls `setup_global(ctx, name, addr, NULL)` (`mir.c:1956-1963`). During `MIR_link`, unresolved imports are looked up in the environment module; if absent and an `import_resolver` exists, MIR calls it and then records the returned address through `MIR_load_external` (`mir.c:1984-2001`).

External symbols are represented in the same global item table used for exported MIR definitions. The `def` argument passed to `setup_global` is `NULL` for external symbols, so these globals have an address but no MIR item definition.

## Ownership And Lifetime

All MIR objects created through the normal API are allocated through `ctx->alloc`. `MIR_finish` owns teardown.

Observed teardown rules:

- Function items free both `func->insns` and `func->original_insns`, destroy `func->vars` and `func->global_vars`, finish the function register table, and free `MIR_func_t` (`mir.c:816-825`).
- Prototype items destroy their arg vector and free `MIR_proto_t` (`mir.c:826-828`).
- Data/bss/ref/lref/expr items free their payload object; if `item->addr != NULL && item->section_head_p`, they also free the loaded data section (`mir.c:833-856`).
- Each `MIR_item_t` and optional `item->data` are freed during item teardown (`mir.c:860-862`).
- Modules are removed from `all_modules`, all items are removed, optional `module->data` bitmap is destroyed, and heap-allocated modules are freed (`mir.c:865-885`).

String and alias names are interned in context string tables. Constructors such as `create_proto`, `MIR_new_data`, and `MIR_new_ref_data` store names with `get_ctx_str` (`mir.c:1190-1200`, `mir.c:1219-1222`, `mir.c:1299-1311`). `MIR_new_str_op` stores string bytes through the context string store (`mir.c:2528-2533`).

Generated code is owned by `machine_code_ctx`. Code holders are allocated from `MIR_mem_map`, published by `_MIR_publish_code`, and unmapped in `code_finish` during `MIR_finish` (`mir.c:4361-4509`). Individual generated functions are not obviously freed independently in the inspected path; generated memory appears context-lifetime scoped.

Instruction operands are stored by value inside `struct MIR_insn` (`mir.h:281-286`). Public operands returned by `MIR_new_*_op` are value objects, not separately allocated handles (`mir.c:2451-2598`).

## Limitations And Trade-Offs

- Module finishing is shallow. `MIR_finish_module` only closes construction. Loading, simplification, import resolution, data allocation, thunk creation, and interface installation happen later.
- `MIR_link` is not just symbol resolution. It simplifies functions, may process inlining, initializes reference/expression data, and installs execution interfaces.
- Function thunks are created at module-load time and redirected later. This unifies interpreter/JIT/lazy interfaces but makes even interpreter operation target-dependent.
- Generated code is managed at context granularity in the observed path. That is simple, but it leaves open whether fine-grained invalidation/freeing exists elsewhere.
- The current native execution model assumes executable memory allocation and target-generated thunks/wrappers.

## Relevance To RISC-V32 / Fantasy Computer Extraction

Context/module/item/function ownership is `essential to MIR semantics`. Thunks, executable code holders, memory protection, and target wrappers are `required for native host execution`. C external resolution and `setjmp` handling are `required only for C ABI compatibility`.

For a fantasy computer subset, a smaller runtime could plausibly keep contexts, modules, items, functions, instruction lists, and direct linking while replacing or removing host thunks, C ABI wrappers, dynamic executable memory, and `setjmp` special handling. A RISC-V32 native JIT would need to preserve the lifecycle contracts but implement target thunks, code cache publication, calling convention lowering, and instruction-cache synchronization for the 32-bit target.

## Open Questions

- Does any public API free a module or generated function before `MIR_finish`?
- What are the exact semantics of `func_redef_permission_p` after functions have generated code?
- Can `MIR_load_module` safely be called multiple times for the same module?
- Which interface callbacks besides interpreter/JIT/lazy are used in practice?
- How much of `MIR_link` simplification is required for interpretation versus only for native generation?
