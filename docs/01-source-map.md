# Source Map

This source map covers the restored MIR checkout at `mir-preservation/git/mir-restored`. It is a first-pass classification, not a full behavioral trace.

Confidence levels:

- `high`: directly indicated by filenames, exported symbols, comments, or project documentation.
- `medium`: supported by sampled symbols and nearby code, but not fully traced.
- `low`: plausible from naming or build layout only; verify before relying on it.

## Core MIR IR Representation

Relevant files:

- `mir.h`
- `mir.c`
- `mir-dlist.h`
- `mir-varr.h`
- `mir-htab.h`
- `mir-hash.h`
- `mir-bitmap.h`
- `mir-reduce.h`

Key structs/types/functions:

- `MIR_context_t`, `struct MIR_context` in `mir.c`
- `MIR_module_t`, `struct MIR_module`
- `MIR_item_t`, `struct MIR_item`, `MIR_item_type_t`
- `MIR_func_t`, `struct MIR_func`
- `MIR_proto_t`, `MIR_data_t`, `MIR_ref_data_t`, `MIR_lref_data_t`, `MIR_expr_data_t`, `MIR_bss_t`
- `MIR_insn_t`, `struct MIR_insn`, `MIR_insn_code_t`
- `MIR_op_t`, `MIR_op_mode_t`, `MIR_mem_t`, `MIR_label_t`, `MIR_var_t`, `MIR_type_t`
- `MIR_new_module`, `MIR_new_func_arr`, `MIR_new_func`, `MIR_new_vararg_func_arr`, `MIR_new_proto_arr`
- `MIR_new_insn_arr`, `MIR_new_insn`, `MIR_new_call_insn`, `MIR_new_label`
- `MIR_new_reg_op`, `MIR_new_int_op`, `MIR_new_mem_op`, `MIR_new_label_op`, `MIR_new_ref_op`
- `MIR_append_insn`, `MIR_insert_insn_before`, `MIR_insert_insn_after`, `MIR_remove_insn`
- `MIR_finish_func`, `MIR_finish_module`, `MIR_finish`

Suspected role:

- `mir.h` defines the public in-memory IR shape: modules contain item lists; function items contain instruction lists; instructions contain opcode plus flexible operand array; operands hold registers, immediates, memory, labels, strings, references, or internal var forms.
- `mir.c` implements construction, validation, finishing, simplification/lowering, module loading/linking, text output, text scanning, binary IO, and executable-code helper functions.
- `mir-dlist.h`, `mir-varr.h`, `mir-htab.h`, `mir-hash.h`, `mir-bitmap.h`, and `mir-reduce.h` provide local container/compression/hash utilities used throughout MIR.

Mechanism classification:

- Module/item/function/instruction/operand/type structures: `essential to MIR semantics`.
- `MIR_OP_VAR`, `MIR_OP_VAR_MEM`, `MIR_USE`, and `MIR_PHI`: `optimization convenience` and internal generator representation, not part of basic textual MIR semantics.
- Hard-register names in variables: `required for native host execution` and `required only for C ABI compatibility` when used to model ABI or global register variables.

Confidence: high.

Questions to verify later:

- Which `MIR_finish_func` transformations mutate user-created instructions, and which only annotate them?
- How are `original_insns` used for preserving/restoring functions after generation or interpretation?
- What invariants are guaranteed after module load versus after function finish?

## MIR Parser / Reader / Writer

Relevant files:

- `mir.c`
- `mir.h`
- `mir-utils/m2b.c`
- `mir-utils/b2m.c`
- `mir-utils/b2ctab.c`
- `mir-bin-driver.c`
- `mir-bin-run.c`
- `MIR.md`

Key structs/types/functions:

- `MIR_scan_string`
- `MIR_output`, `MIR_output_module`, `MIR_output_item`, `MIR_output_insn`, `MIR_output_op`
- `MIR_write`, `MIR_write_module`, `MIR_write_with_func`, `MIR_write_module_with_func`
- `MIR_read`, `MIR_read_with_func`
- `struct io_ctx`, `enum token_code`, `struct token`, `struct scan_ctx`, `struct label_desc` in `mir.c`
- Binary IO helpers and tags in `mir.c` near `MIR_write_*` and `MIR_read_with_func`

Suspected role:

- Textual MIR is assembler-like and scanned by `MIR_scan_string` in `mir.c`.
- Binary MIR is written/read by `MIR_write*` and `MIR_read*`, using compact tags and the `mir-reduce.h` reducer.
- `mir-utils/m2b.c` and `mir-utils/b2m.c` convert between text and binary forms.
- `mir-bin-run.c` appears to execute binary MIR modules, including interpreter/JIT/lazy execution selection described in `README.md`.

Mechanism classification:

- Textual/API construction into in-memory MIR: `essential to MIR semantics`.
- Binary MIR: likely `optimization convenience` for compact/faster IO, not required for semantics.
- `mir-bin-run` host execution wrapper: `required for native host execution`.

Confidence: high for API names and file roles; medium for detailed binary format until traced.

Questions to verify later:

- Does the scanner produce any AST-like intermediate, or does it construct MIR items and instruction lists directly?
- Are binary MIR files versioned or self-describing enough for archival use?
- What semantic checks are shared between text scanning, binary reading, and API construction?

## Public API

Relevant files:

- `mir.h`
- `mir-gen.h`
- `c2mir/c2mir.h`
- `llvm2mir/llvm2mir.h`
- `mir2c/mir2c.h`
- `mir-alloc.h`
- `mir-code-alloc.h`
- `CUSTOM-ALLOCATORS.md`
- `MIR.md`

Key structs/types/functions:

- `MIR_init`, `MIR_init2`, `_MIR_init`, `MIR_finish`
- Construction APIs listed in `mir.h`
- Linking/execution APIs: `MIR_load_module`, `MIR_load_external`, `MIR_link`, `MIR_interp`, `MIR_set_interp_interface`
- Generator APIs: `MIR_gen_init`, `MIR_gen_set_debug_file`, `MIR_gen_set_debug_level`, `MIR_gen_set_optimize_level`, `MIR_gen`, `MIR_gen_finish`
- Allocator types: `MIR_alloc_t`, `MIR_code_alloc_t`

Suspected role:

- `mir.h` is the main public API and also exposes internal `_MIR_*` helpers behind `#ifdef MIR_INTERNAL`.
- `mir-gen.h` is the public generator/JIT API.
- `c2mir.h`, `llvm2mir.h`, and `mir2c.h` expose frontend/conversion APIs.
- `mir-alloc.h` and `mir-code-alloc.h` support custom heap and executable-code allocation.

Mechanism classification:

- API construction and context lifetime: `essential to MIR semantics`.
- Generator API: `required for native host execution`.
- Code allocator API: `required for native host execution`; possibly `removable for a fantasy computer subset` if code memory is static or non-W^X.

Confidence: high.

Questions to verify later:

- Which `_MIR_*` symbols are intentionally semi-public for targets/frontends?
- What API stability guarantee exists beyond `MIR_API_VERSION`?
- Are contexts independent enough for thread-local use only, or is shared module migration common?

## Interpreter

Relevant files:

- `mir-interp.c`
- `mir.h`
- Machine support files: `mir-x86_64.c`, `mir-aarch64.c`, `mir-ppc64.c`, `mir-s390x.c`, `mir-riscv64.c`
- `HOW-TO-PORT-MIR.md`
- `mir-tests/*-interp.c`
- `c-tests/use-c2m-interp`, `c-tests/use-c2m-bin-interp`, `c-tests/use-l2m-interp`

Key structs/types/functions:

- `MIR_interp`, `MIR_interp_arr`, `MIR_interp_arr_varg`, `MIR_set_interp_interface`
- `generate_icode`, `finish_func_interpretation`, `interp_init`, `interp_finish`
- `struct interp_ctx`, `struct func_desc`, `code_t`, `MIR_val_t`
- `call`, `call_insn_execute`, `_MIR_get_ff_call`
- Target hooks: `_MIR_get_bstart_builtin`, `_MIR_get_bend_builtin`, `_MIR_get_interp_shim`, `_MIR_get_thunk`, `_MIR_redirect_thunk`

Suspected role:

- The interpreter converts MIR functions into an internal instruction-code array (`code_t` is `MIR_val_t *`) and dispatches over that internal code.
- Calls to external C functions and MIR functions require target-provided shims/thunks so interpreted functions can share a C-callable interface with generated code.
- The interpreter is not purely target-independent; `HOW-TO-PORT-MIR.md` requires `mir-<target>.c` before interpreter and C2MIR interpreter use.

Mechanism classification:

- Interpreting MIR operations: `essential to MIR semantics` for non-JIT execution.
- Interp shims, varargs helpers, and foreign-function calls: `required only for C ABI compatibility`.
- Target thunks used to expose interpreted MIR as C-callable functions: `required for native host execution`.
- A fantasy computer subset could potentially remove C ABI shims if all calls are internal and the runtime owns the call convention: `removable for a fantasy computer subset`.

Confidence: medium-high.

Questions to verify later:

- What exact dispatch model is used in `mir-interp.c`?
- Which MIR instructions are rejected or limited by the interpreter?
- Does interpreter execution preserve enough behavior to be a correctness oracle for all supported backends?

## JIT Generator Common Code

Relevant files:

- `mir-gen.c`
- `mir-gen.h`
- `mir.h`
- `mir-code-alloc.h`
- `mir-code-alloc-default.c`
- Machine-specific generator files included by `mir-gen.c`
- `mir-gen.svg`

Key structs/types/functions:

- `MIR_gen_init`, `MIR_gen`, `MIR_set_gen_interface`, `MIR_set_lazy_gen_interface`, `MIR_set_lazy_bb_gen_interface`, `MIR_gen_finish`
- `struct gen_ctx`, `struct func_cfg`, `bb_t`, `edge_t`, `bb_insn_t`
- `generate_func_code`
- `build_func_cfg`, `target_machinize`, `target_make_prolog_epilog`, `target_translate`, `target_rebase`
- `generate_bb_version_machine_code`, `bb_version_generator`
- `_MIR_publish_code`, `_MIR_get_new_code_addr`, `_MIR_publish_code_by_addr`, `_MIR_change_code`, `_MIR_update_code_arr`

Suspected role:

- `mir-gen.c` contains backend-independent CFG construction, optional optimization passes, register allocation, common control flow for whole-function generation, and lazy basic-block versioning support.
- Target files are included into `mir-gen.c` through preprocessor selection, providing target-specific macros/functions used by common code.
- `MIR_gen` explicitly generates machine code for a function. `MIR_set_lazy_gen_interface` installs wrappers to generate on first call. `MIR_set_lazy_bb_gen_interface` appears to support basic-block versioning.

Mechanism classification:

- Whole-function code generation: `required for native host execution`.
- Lazy generation wrappers: `optimization convenience` and `required for native host execution` in the current runtime.
- Lazy basic-block versioning: likely `optimization convenience`; may be `removable for a fantasy computer subset`.
- CFG and lowering needed for code generation: `required for native host execution`; only partly `essential to MIR semantics`.

Confidence: medium-high.

Questions to verify later:

- What exact units are compiled by each interface: full function, lazy full function, or basic-block versions?
- Which generator mutations are temporary and restored, and which become permanent in the function item?
- How does `machine_code_p` alter the pipeline in `generate_func_code`?

## Optimization Passes

Relevant files:

- `mir-gen.c`
- `mir.c`
- Machine-specific `mir-gen-*.c` files for target lowering and combine/code selection.

Key structs/types/functions:

- Top-of-file optimization pipeline comment in `mir-gen.c`
- `MIR_gen_set_optimize_level`
- `build_func_cfg`, `build_ssa`, `addr_transform`, `clone_bbs`
- `gvn`, `copy_propagation`, `dse`, `ssa_dead_code_elimination`, `licm`, `pressure_relief`, `ssa_combine`, `out_of_ssa`
- `jump_opt`, `combine`, `dead_code_elimination`
- `target_machinize`, `target_split_insns`
- Simplification/lowering routines in `mir.c`, including `MIR_finish_func`

Suspected role:

- `mir.c` performs always-on MIR simplification/lowering during function finalization.
- `mir-gen.c` implements optimization levels: comment states `0: fast gen`, `1: RA+combiner`, `2: +GVN/CCP (default)`, `>=3: everything`.
- `-O2` and above enable SSA, GVN/constant propagation/redundant load elimination, copy propagation, DSE, DCE, LICM, pressure relief, SSA combine, and out-of-SSA.
- `-O1` and above enable post-RA combine and DCE.

Mechanism classification:

- Simplification needed to execute all MIR forms: likely `essential to MIR semantics` or execution precondition; verify per transformation.
- SSA/GVN/DSE/LICM/pressure relief/combine: `optimization convenience`.
- Target machinization: `required for native host execution` and often `required only for C ABI compatibility`.

Confidence: medium-high from `mir-gen.c` comments; detailed pass behavior not yet verified.

Questions to verify later:

- What does `>=3` add beyond `-O2`?
- What cost model does each pass use, especially block cloning and pressure relief?
- What optimizations are deliberately omitted to preserve compile latency?

## Register Allocation

Relevant files:

- `mir-gen.c`
- `mir-gen-*.c`
- `HOW-TO-PORT-MIR.md`

Key structs/types/functions:

- `reg_alloc`, `build_live_ranges`, `build_conflict_matrix`, `coalesce`, `assign`, `rewrite`, `split`
- `struct ra_ctx`, `struct lr_ctx`, `live_range_t`, `lr_gap_t`
- `target_hard_reg_type_ok_p`, `target_fixed_hard_reg_p`, `target_call_used_hard_reg_p`
- `target_locs_num`, `target_nth_loc`, `target_get_stack_slot_offset`, `target_get_stack_slot_base_reg`
- `MAX_HARD_REG`, `SP_HARD_REG`, `HARD_REG_FRAME_POINTER`, `TEMP_*_HARD_REG*`

Suspected role:

- Common code uses live ranges, conflict matrices, coalescing, priority-based linear-scan allocation, stack slots, and optional live-range splitting.
- Target code describes hard registers, fixed/call-used registers, register classes by type, stack slot layout, and addressing constraints.

Mechanism classification:

- Register allocation itself: `required for native host execution`.
- Specific hard-register sets and stack slot offsets: `backend-specific engineering detail`.
- Coalescing and splitting: `optimization convenience`.
- For a fantasy VM interpreter-only subset, register allocation is `removable for a fantasy computer subset`.

Confidence: medium-high.

Questions to verify later:

- How does MIR distinguish pseudo-register numbers from hard-register numbers internally after CFG build?
- Which spill/reload strategy is used on each target?
- How much of register allocation assumes 64-bit stack slots?

## Backend Abstraction

Relevant files:

- `mir-gen.c`
- `mir-gen-x86_64.c`
- `mir-gen-aarch64.c`
- `mir-gen-ppc64.c`
- `mir-gen-s390x.c`
- `mir-gen-riscv64.c`
- `mir-gen-stub.c`
- `HOW-TO-PORT-MIR.md`

Key structs/types/functions:

- Target hooks named in `HOW-TO-PORT-MIR.md`: `target_init`, `target_finish`, `target_machinize`, `target_make_prolog_epilog`, `target_translate`, `target_rebase`, `target_change_to_direct_calls`
- Register hooks: `target_hard_reg_type_ok_p`, `target_fixed_hard_reg_p`, `target_call_used_hard_reg_p`, `target_locs_num`
- Instruction legality/selection hooks: `target_insn_ok_p`, `target_memory_ok_p`, `target_split_insns`, `target_get_early_clobbered_hard_regs`
- Basic-block versioning hooks: `target_bb_translate_start`, `target_bb_insn_translate`, `target_bb_translate_finish`, `target_bb_rebase`

Suspected role:

- There is no separate backend interface object. The common generator expects target files to define a set of macros, constants, and static functions before `mir-gen.c` uses them.
- Target files are selected with architecture preprocessor branches inside `mir-gen.c`.

Mechanism classification:

- Backend hook contract: `required for native host execution`.
- Source-level include strategy: `backend-specific engineering detail`.
- For a fantasy computer subset, a smaller hook surface might be possible if ABI and code generation are simplified: `removable for a fantasy computer subset` for many hooks.

Confidence: medium.

Questions to verify later:

- Which target hooks are mandatory versus only used when a feature is enabled?
- Is `mir-gen-stub.c` a no-JIT fallback or a build placeholder?
- How much does the hook contract assume a real C ABI?

## Machine-Specific Backends

Relevant files:

- Common/interpreter target support: `mir-x86_64.c`, `mir-aarch64.c`, `mir-ppc64.c`, `mir-s390x.c`, `mir-riscv64.c`
- JIT generator targets: `mir-gen-x86_64.c`, `mir-gen-aarch64.c`, `mir-gen-ppc64.c`, `mir-gen-s390x.c`, `mir-gen-riscv64.c`, `mir-gen-stub.c`
- Target headers: `mir-x86_64.h`, `mir-aarch64.h`, `mir-ppc64.h`, `mir-s390x.h`, `mir-riscv64.h`
- C2MIR target dirs: `c2mir/x86_64/`, `c2mir/aarch64/`, `c2mir/ppc64/`, `c2mir/s390x/`, `c2mir/riscv64/`

Key structs/types/functions:

- `_MIR_get_thunk`, `_MIR_redirect_thunk`, `_MIR_get_wrapper`, `_MIR_get_interp_shim`, `_MIR_get_ff_call`
- `_MIR_get_bstart_builtin`, `_MIR_get_bend_builtin`
- `_MIR_replace_bb_thunk`, `_MIR_get_bb_wrapper`, `_MIR_get_bb_thunk`
- Target generator functions listed in the previous category.
- RISC-V64 target symbols sampled from `mir-gen-riscv64.c`: `machinize_call`, `target_machinize`, `target_make_prolog_epilog`, `target_translate`, `target_rebase`, `target_bb_rebase`

Suspected role:

- `mir-<target>.c` files provide target support shared by interpreter and generator, especially thunks, wrappers, vararg/foreign-function handling, and basic-block thunk patching.
- `mir-gen-<target>.c` files transform MIR to target-constrained MIR/machine-like instructions, perform prolog/epilog construction, encode instructions, and patch/rebase emitted code.
- Existing supported native generator targets include x86_64, aarch64, ppc64, s390x, and riscv64. No RISC-V32 backend was observed in this pass.

Mechanism classification:

- Target instruction encoding and relocation: `backend-specific engineering detail`.
- Thunks/wrappers and FFI helpers: `required for native host execution`, often `required only for C ABI compatibility`.
- RISC-V64 backend as reference for RISC-V32: relevant but not semantically binding; much of it is ABI and XLEN-specific.

Confidence: high for file inventory; medium for detailed backend roles.

Questions to verify later:

- Which backend is the simplest reliable reference for a new target?
- How much of `mir-riscv64.c` can survive for RISC-V32, given pointer width, register ABI, and instruction encoding differences?
- Are all listed backends equally mature, or do tests/release notes show different confidence?

## Runtime Memory Management

Relevant files:

- `mir-alloc.h`
- `mir-alloc-default.c`
- `mir-code-alloc.h`
- `mir-code-alloc-default.c`
- `mir.c`
- Machine-specific files that publish, patch, or flush executable code.

Key structs/types/functions:

- `MIR_alloc_t`, default allocator in `mir-alloc-default.c`
- `MIR_code_alloc_t`, `MIR_mem_map`, `MIR_mem_unmap`, `MIR_mem_protect`
- `MIR_mem_protect_t` with `PROT_WRITE_EXEC` and `PROT_READ_EXEC`
- `_MIR_publish_code`, `_MIR_get_new_code_addr`, `_MIR_publish_code_by_addr`, `_MIR_set_code`
- `_MIR_change_code`, `_MIR_update_code`, `_MIR_update_code_arr`
- `struct code_holder`, `struct machine_code_ctx` in `mir.c`
- Platform calls: `mmap`, `mprotect`, `munmap`, `VirtualAlloc`, `VirtualProtect`, `VirtualFree`, Apple `MAP_JIT`, `pthread_jit_write_protect_np`, `sys_icache_invalidate`

Suspected role:

- Heap allocation and executable-code allocation are separable through `MIR_init2`.
- Default code allocation maps executable memory and toggles write/execute protection around publication and patching.
- `mir.c` stores generated code chunks and handles relocation/patching through `_MIR_set_code` and update helpers.

Mechanism classification:

- Normal object allocation: `essential to MIR semantics`.
- Executable memory mapping/protection: `required for native host execution`.
- W^X and platform JIT APIs: `backend-specific engineering detail` and OS detail.
- For a fantasy computer subset, dynamic OS code allocation may be `removable for a fantasy computer subset`.

Confidence: medium-high.

Questions to verify later:

- Does MIR ever free individual generated functions, or only release code with context finalization?
- Are generated code blocks immutable after publication except explicit patch helpers?
- What instruction-cache flushing is done on non-Apple architectures?

## C2MIR Frontend

Relevant files:

- `c2mir/c2mir.c`
- `c2mir/c2mir.h`
- `c2mir/c2mir-driver.c`
- `c2mir/README.md`
- `c2mir/mirc.h` and companion headers
- Target directories: `c2mir/x86_64/`, `c2mir/aarch64/`, `c2mir/ppc64/`, `c2mir/s390x/`, `c2mir/riscv64/`
- `c-tests/`
- `csmith-c2m.sh`, `csmith-c2m-gcc.sh`

Key structs/types/functions:

- To be inspected in a later pass. Porting guide names target ABI hooks such as `target_init_arg_vars`, `target_return_by_addr_p`, `target_add_res_proto`, `target_add_call_res_op`, `target_gen_post_call_res_code`, `target_add_ret_ops`, `target_get_blk_type`, `target_add_arg_proto`, `target_add_call_arg_op`, and `target_gen_gather_arg`.

Suspected role:

- C2MIR parses/compiles C into MIR and contains target-specific C ABI lowering and predefined headers.
- It is separate from core MIR semantics but important because many ABI and type-layout constraints enter MIR through C2MIR output.

Mechanism classification:

- C frontend: not `essential to MIR semantics`.
- C ABI lowering and predefined headers: `required only for C ABI compatibility`.
- For a fantasy computer subset, C2MIR may be partially or fully `removable for a fantasy computer subset`.

Confidence: medium.

Questions to verify later:

- Which C features are unsupported or lowered approximately?
- Does C2MIR assume 64-bit targets in places that block RISC-V32?
- Which target ABI code is reusable for a non-C fantasy language?

## Tests

Relevant files/directories:

- `mir-tests/`
- `adt-tests/`
- `c-tests/`
- `.github/workflows/`
- `check-threads.sh`
- `csmith-c2m.sh`, `csmith-c2m-gcc.sh`

Key structs/types/functions:

- `mir-tests/run-test.c`
- MIR API examples/tests: `api-loop.h`, `api-memop.h`, `api-sieve.h`
- Interpreter tests: `args-interp.c`, `hi-interp.c`, `loop-interp.c`, `sieve-interp.c`
- Generator tests: `loop-sieve-gen.c`, `simplify.c`
- Text MIR fixtures: `test1.mir` through `test16.mir`
- ADT tests for bitmap, dlist, htab, mp, reduce, varr.

Suspected role:

- Tests cover core data structures, MIR text/API construction, interpreter behavior, generator behavior, C2MIR output, and LLVM2MIR where available.
- CI badges in `README.md` indicate tests on x86_64, Apple aarch64, aarch64, ppc64le, s390x, and riscv64.

Mechanism classification:

- Tests are archival verification assets, not runtime mechanisms.
- Test coverage is essential to preservation work before redesign.

Confidence: high for inventory; medium for coverage claims until test targets are read.

Questions to verify later:

- Which tests exercise binary MIR versus textual MIR?
- Which tests run on each architecture in CI?
- Are there tests for lazy basic-block generation?

## Benchmarks

Relevant files/directories:

- `c-benchmarks/`
- `sieve.c`
- Benchmark targets in `GNUmakefile`
- GitHub benchmark workflow indicated in `README.md`

Key structs/types/functions:

- `c-benchmarks/run-benchmarks.sh`
- Benchmark programs such as `array.c`, `binary-trees.c`, `funnkuch-reduce.c`, `hash.c`, `heapsort.c`, `mandelbrot.c`, `nbody.c`, `spectral-norm.c`, `oggenc.c`

Suspected role:

- Benchmarks compare C2MIR/interpreter/JIT behavior and performance against expected outputs.
- They are useful for understanding latency/quality trade-offs but are not semantic specifications by themselves.

Mechanism classification:

- Benchmarks are preservation evidence and performance context.

Confidence: medium.

Questions to verify later:

- Which benchmark modes are officially compared?
- Are benchmark results preserved in the archive or only scripts?
- Do benchmarks reveal deliberate trade-offs between `-O0`, `-O1`, `-O2`, and `-O3`?

## Build System

Relevant files:

- `GNUmakefile`
- `CMakeLists.txt`
- `.travis.yml`
- `.github/workflows/`
- `INSTALL.md`
- `check-threads.sh`

Key structs/types/functions:

- Make targets: `all`, `debug`, `debug2`, `test`, `bench`, install/uninstall targets
- Build artifacts: `libmir.a`, shared library, `c2m`, `m2b`, `b2m`, `b2ctab`, `l2m`, `mir-bin-run`
- Build flags: `MIR_NO_GEN_DEBUG`, `MIR_INTERP_TRACE`, `C2MIR_PARALLEL`, sanitizer flags in debug builds

Suspected role:

- `GNUmakefile` is the primary build description observed in this pass.
- `CMakeLists.txt` provides alternate build integration.
- Build system selects OS/compiler flags, creates libraries and CLI utilities, and detects optional pthread/LLVM support.

Mechanism classification:

- Build system is not MIR semantics but is required to reproduce and verify the preserved code.
- Platform flags and executable/shared library setup are `required for native host execution`.

Confidence: high.

Questions to verify later:

- Which build paths are actively maintained?
- Does CMake build all tools and tests or only the core library?
- What host/target combinations are represented in CI versus only documented?

## Documentation / Examples

Relevant files:

- `README.md`
- `MIR.md`
- `HOW-TO-PORT-MIR.md`
- `INSTALL.md`
- `CUSTOM-ALLOCATORS.md`
- `c2mir/README.md`
- `llvm2mir/README.md`
- `mir-utils/README.md`
- `mir-tests/readme-example.c`
- `mir-tests/*.mir`
- `mir-gen.svg`, `c2mir/c2mir.svg`, `mir3.svg`, `mirall.svg`
- Preserved web/slides under `mir-preservation/web/`

Key structs/types/functions:

- Documentation references the API functions and target hooks listed above.
- Examples show `MIR_load_module`, `MIR_load_external`, `MIR_link`, `MIR_set_interp_interface`, `MIR_set_gen_interface`, `MIR_set_lazy_gen_interface`, `MIR_gen`, and `MIR_interp`.

Suspected role:

- Existing docs provide author-authored descriptions of MIR syntax, API, running modes, custom allocators, and porting responsibilities.
- Diagrams may help reconstruct pipeline and frontend relationships, but should be checked against code.

Mechanism classification:

- Documentation is preservation evidence; source remains authoritative.

Confidence: high.

Questions to verify later:

- Which author docs predate later source changes?
- Do diagrams match the current restored revision?
- Which preserved articles contain design rationale not visible in code comments?

## Cross-Cutting Initial Observations

- `AST` is probably not the right term for the core in-memory MIR representation. Observed structures are modules, items, functions, instruction lists, operands, and labels. The text scanner may have token structures, but the main output appears to be MIR IR objects rather than a retained syntax tree. This remains to be verified by tracing `MIR_scan_string`.
- The JIT appears to support complete function generation and lazy generation. `mir-gen.c` also contains lazy basic-block versioning symbols, but this pass does not yet document that path.
- RISC-V64 support exists; RISC-V32 support was not observed.
- Real host execution introduces a large amount of target/ABI/runtime machinery that may not be essential to MIR semantics.

## Open Questions

- What is the best minimal call path to trace for each construction mode: API, text, and binary?
- Which files should be read first for exact ownership/lifetime rules in `MIR_context_t`?
- Are there preserved issue/discussion notes relevant to RISC-V32 or constrained runtimes?
- Which backend has the clearest, least special-case architecture for documentation purposes?
