# Whole-Function JIT Pipeline

## Entry Point

The public whole-function entry point is `MIR_gen(ctx, func_item)`, declared in `mir-gen.h:23` and implemented as `generate_func_code(ctx, func_item, TRUE)` (`mir-gen.c:9505-9506`).

`generate_func_code` is the central generation pipeline (`mir-gen.c:9277-9502`). It handles an already-generated function as a fast path, otherwise it duplicates the MIR instruction list, builds a CFG, runs optimization/lowering/register allocation, emits target machine code, publishes executable code, redirects the function thunk, restores the original MIR instruction list, and stores the generated-code pointer.

## Compilation Unit

For normal generation, the compilation unit is one complete MIR function.

Observed evidence:

- The function asserts `func_item->item_type == MIR_func_item` (`mir-gen.c:9285`).
- It operates through `curr_func_item`, `func_item->u.func->insns`, and one `struct func_cfg` attached temporarily to `func_item->data` (`mir-gen.c:9302-9305`).
- `MIR_gen` always passes `machine_code_p == TRUE`, which enables whole-function `target_translate` and publication (`mir-gen.c:9474-9502`).

`machine_code_p` is the internal switch between complete function code generation and preparatory generation for lazy basic-block mode. When false, the same preparatory pipeline runs but the whole-function translate/publish/restore/store steps are skipped (`mir-gen.c:9474-9502`).

## Generator Context

`struct gen_ctx` is allocated by `MIR_gen_init` and stored through the context's `gen_ctx` slot (`mir-gen.c:207-249`, `mir-gen.c:9641-9705`).

Observed fields include:

- Current state: `ctx`, `optimize_level`, `curr_func_item`, `curr_cfg`, current BB/loop indices.
- Debug state: `debug_file`, `debug_level` when generator debug is enabled.
- Temporary memory and vectors: `to_free`, `temp_ops`, `temp_insns`, `temp_bb_insns`, loop work vectors, memory attributes.
- Analysis bitmaps: tied registers, address registers, instruction-consideration bitmaps, call-used hard registers, function-used hard registers.
- Phase subcontexts: target, data-flow, SSA, GVN, live ranges, coalescing, register allocation, and combine.
- Target/register accounting: maximum integer/floating hard registers and function stack slot count.
- Lazy BB state: successor BB version vectors, successor address vectors, BB wrapper, and property-attribute maps.

`MIR_gen_init` sets the default optimization level to `2`, creates vectors/bitmaps, initializes phase subcontexts, initializes the target backend, computes call-used hard-register bitmaps, creates the lazy BB wrapper, and clears BB statistics (`mir-gen.c:9641-9705`).

`MIR_gen_finish` destroys phase subcontexts, target state, bitmaps, vectors, temporary allocations, and the generator context itself (`mir-gen.c:9707-9752`).

## Function CFG

`struct func_cfg` stores function-level CFG state: `max_var`, current BB instruction index, per-register info, `call_crossed_regs`, a list of BBs, and root loop node (`mir-gen.c:495-502`).

`struct bb` stores CFG block state: order indices, incoming/outgoing edges, a list of `bb_insn` nodes, call/reachability flags, data-flow bitmaps, loop node, and pressure data (`mir-gen.c:437-451`). `struct edge` records source/destination blocks and fall-through/back-edge flags (`mir-gen.c:402-407`). `struct bb_insn` wraps a MIR instruction with per-pass metadata such as indexes, GVN data, memory index, owning BB, dead variables, call hard-register arguments, and label displacement (`mir-gen.c:422-433`).

`build_func_cfg` initializes the CFG, computes maximum variable numbers from function variables, creates entry/exit blocks, prepends a synthetic label, tracks address instructions, inserts global-variable `MIR_USE` before returns, expands some overflow operations according to target macros, transforms `MIR_OP_REG` and `MIR_OP_MEM` operands into internal variable forms, creates edges for branches/labels/fall-through, handles indirect jumps by adding possible label edges, marks label-address targets reachable, removes unreachable blocks at optimization levels above zero, adds entry/exit edges, enumerates BBs, and creates register-info/liveness support storage (`mir-gen.c:1571-1887`).

`destroy_func_cfg` removes per-instruction data, deletes BBs, destroys register info and bitmaps, frees `func_item->data`, and clears it back to `NULL` (`mir-gen.c:1889-1912`).

## MIR-To-Machine Lowering

The backend-independent pipeline performs MIR-level analysis and transformations before target lowering. The first target-specific lowering call in the main pipeline is `target_machinize(gen_ctx)` (`mir-gen.c:9421`).

Observed sequence around lowering:

1. Duplicate original instructions with `_MIR_duplicate_func_insns` (`mir-gen.c:9302-9303`).
2. Build CFG (`mir-gen.c:9304-9306`).
3. Run optional MIR-level optimization depending on `optimize_level` (`mir-gen.c:9311-9420`).
4. Call `target_machinize` and then `make_io_dup_op_insns` (`mir-gen.c:9421-9422`).
5. Perform liveness, register allocation, combine, dead-code cleanup, prolog/epilog construction, target splitting, and final target translation (`mir-gen.c:9427-9477`).

Classification:

- CFG construction and variable normalization: `required for native host execution` in current MIR generator.
- Target machinization: `backend-specific engineering detail`.
- MIR-level optimization before target lowering: `optimization convenience`, except where required to normalize unsupported input patterns for later phases.

## Optimization Sequence By Level

The generator default is `optimize_level = 2` (`mir-gen.c:9648-9650`). `struct gen_ctx` summarizes levels as `0: fast gen; 1: RA+combiner; 2: +GVN/CCP (default); >=3: everything` (`mir-gen.c:207-210`). The manual describes levels in `MIR.md:763-772`.

Observed code-level sequence:

- Level 0: builds CFG, lowers to machine-oriented MIR, calculates live info, runs register allocation, makes prolog/epilog, splits target instructions, translates, publishes code.
- Level >= 1: builds loop tree for later phases, runs post-RA `combine`, and post-combine dead-code elimination (`mir-gen.c:9427-9430`, `mir-gen.c:9452-9467`).
- Level >= 2: BB cloning, SSA construction, address transformation when needed, GVN, copy propagation, DSE, SSA dead-code elimination, loop tree/LICM, pressure relief, conventional SSA conversion, SSA combine, SSA destruction, jump optimization, move collection, optional coalescing (`mir-gen.c:9311-9420`, `mir-gen.c:9431-9442`).

The manual says level 1 is faster and still improves compactness/speed, level 2 adds common subexpression elimination and sparse conditional constant propagation, and level 3 adds register renaming and loop invariant code motion (`MIR.md:763-772`). The inspected code gates LICM at `>= 2` (`mir-gen.c:9375-9386`), so the manual/code mapping should be verified before relying on that summary for exact pass gating.

## Register Allocation

Before register allocation, the pipeline:

- optionally coalesces moves at level >= 2 (`mir-gen.c:9431-9442`);
- considers all live variables (`mir-gen.c:9443`);
- calculates function CFG live information (`mir-gen.c:9444`);
- then calls `reg_alloc(gen_ctx)` (`mir-gen.c:9446`).

Register-allocation state is initialized in `MIR_gen_init` through `init_ra(gen_ctx)` (`mir-gen.c:9684`) and destroyed by `finish_ra(gen_ctx)` (`mir-gen.c:9719`). The generator context also stores hard-register bitmaps and target-derived call-used hard-register sets (`mir-gen.c:220-221`, `mir-gen.c:9692-9702`).

## Prolog/Epilog

After register allocation and optional combine/dead-code cleanup, the pipeline calls:

- `target_make_prolog_epilog(gen_ctx, func_used_hard_regs, func_stack_slots_num)` (`mir-gen.c:9468`);
- `target_split_insns(gen_ctx)` (`mir-gen.c:9469`).

This places stack frame and ABI details in the backend-specific layer. It is `required for native host execution` and usually `required only for C ABI compatibility` when the generated code must be callable as a C function.

## Code Emission

Whole-function machine-code emission happens only when `machine_code_p` is true:

- `target_translate(gen_ctx, &code_len)` returns generated bytes (`mir-gen.c:9474-9475`).
- `_MIR_publish_code(ctx, code, code_len)` copies those bytes into executable code memory and returns the executable address (`mir-gen.c:9475-9477`, `mir.c:4426-4434`).
- `_MIR_publish_code` uses context-owned `machine_code_ctx` code holders (`mir.c:4353-4364`).
- `_MIR_set_code` temporarily changes memory protection to write/execute, copies code or relocation values, and changes protection back to read/execute (`mir.c:4398-4409`).
- Code publication flushes the instruction cache with `_MIR_flush_code_cache` (`mir.c:4412-4423`).

The code-holder model is context-lifetime scoped. Code holders are unmapped in `code_finish`, which is called during `MIR_finish` (`mir.c:4492-4505`).

## Relocation/Rebase

After publication, `generate_func_code` calls `target_rebase(gen_ctx, func_item->u.func->call_addr)` (`mir-gen.c:9475-9478`). This is the target hook that applies or finalizes machine-code addresses relative to the published base.

Runtime code patching utilities are declared in `mir.h:682-727` and implemented in `mir.c:4391-4489`. Relevant functions:

- `_MIR_publish_code`: allocate and publish new code.
- `_MIR_get_new_code_addr`: reserve/peek at the next code address.
- `_MIR_publish_code_by_addr`: publish only if the expected next address matches.
- `_MIR_set_code`: write relocation values/code bytes under appropriate page protections.
- `_MIR_change_code` / `_MIR_update_code_arr`: patch existing code and flush the instruction cache.

## Code Publication

`_MIR_publish_code` is marked with a `thread safe` comment (`mir.c:4426-4428`), but no lock is visible in the inspected implementation. It calls `get_last_code_holder`, which mutates the code-holder vector and `free` pointer (`mir.c:4369-4388`), then `add_code`, which writes code and advances the free pointer (`mir.c:4412-4423`).

Open question: thread-safety may be delegated to `MIR_mem_map`, allocator policy, platform assumptions, or missing locks elsewhere. The code comment alone should not be treated as a complete concurrency proof.

## Function Call Address Update

After publication:

- `func_item->u.func->call_addr` is set to the published machine-code address (`mir-gen.c:9475-9477`).
- Optional call tracing can replace `call_addr` with a wrapper while leaving `machine_code` as the raw body (`mir-gen.c:9478-9480`).
- `_MIR_redirect_thunk(ctx, func_item->addr, func_item->u.func->call_addr)` redirects the public function thunk (`mir-gen.c:9481-9485`).
- `_MIR_restore_func_insns(ctx, func_item)` restores the original MIR instruction list (`mir-gen.c:9499`).
- `func_item->u.func->machine_code = machine_code` records that generation is complete (`mir-gen.c:9500-9501`).

The comment before storing `machine_code` says an atomic operation should be used but C2MIR does not implement atomics yet (`mir-gen.c:9500`). This is a concrete concurrency limitation.

## Limitations And Trade-Offs

- The normal JIT compiles complete functions. It is not a trace compiler or region compiler.
- The pipeline mutates a duplicated instruction list during generation and restores the original only after successful whole-function emission. This keeps archived MIR IR stable after generation but makes generation internally stateful.
- The optimization pipeline is latency-conscious: level 0 and 1 exist for faster generation, while level 2 is default and higher levels add cost.
- Code publication is context-lifetime scoped in the observed implementation; individual generated functions are not independently freed.
- Native execution assumes executable memory management, writable patch windows, and instruction-cache flushing.
- Thread-safety is uncertain around generation and code publication despite comments on some code-writing helpers.

## Relevance To RISC-V32 / Fantasy Computer Extraction

- Function-level compilation unit: `optimization convenience` and `required for native host execution` in current MIR, but not essential to MIR semantics.
- CFG and variable normalization: useful for any compiler backend; likely `essential` only to a MIR-inspired compiler, not to MIR IR itself.
- Target machinization, prolog/epilog, register allocation, code emission, relocation/rebase: `backend-specific engineering detail`.
- C-callable stack frame and ABI lowering: `required only for C ABI compatibility`.
- Code cache, W^X-style protection transitions, and instruction-cache flushing: `required for native host execution`.
- Whole-function generated-code storage in `func_item`: removable for interpreter-only or fantasy-bytecode subsets.

For RISC-V32, the existing RISC-V generator is explicitly RV64-only (`mir-gen.c:321-329`). A RISC-V32 backend would need new hard-register definitions, 32-bit pointer and integer lowering rules, calling convention support, stack-frame layout, relocation/rebase logic, thunks/wrappers, and instruction-cache handling.

## Open Questions

- What exact target hooks are mandatory for a new backend versus optional optimizations?
- Which pass actually implements sparse conditional constant propagation, and how does that correspond to the manual's level descriptions?
- Are `_MIR_publish_code` and `_MIR_change_code` safe under concurrent lazy generation?
- How does generated code handle multiple return values and block values at the ABI boundary on each backend?
- Can failed generation leave duplicated or transformed instructions installed in the function?
