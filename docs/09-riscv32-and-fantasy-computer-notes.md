# RISC-V32 And Fantasy Computer Notes

## Existing RISC-V64 Support

Fact: The current generator supports RISC-V only through the RV64 backend include path. In `mir-gen.c`, RISC-V generation requires `__riscv_xlen == 64`, floating-point length at least 64, double-float ABI, multiply, divide, and compressed instructions; otherwise compilation fails with `#error "only 64-bit RISCV supported (at least rv64imafd)"` (`mir-gen.c:321-329`). It also rejects `__riscv_flen == 128` (`mir-gen.c:326-327`).

Relevant files:

- `mir-riscv64.h`: shared register names, hard-register predicates, temporary hard registers, RISC-V immediate helpers (`mir-riscv64.h:1-78`).
- `mir-riscv64.c`: runtime/ABI helpers, thunks, wrappers, vararg helpers, and lazy-BB wrapper code (`mir-riscv64.c:1-192`, `mir-riscv64.c:852-937`, `mir-riscv64.c:1092-1177`).
- `mir-gen-riscv64.c`: target lowering, ABI machinization, pattern matching, machine-code emission, rebasing, and lazy-BB target hooks (`mir-gen-riscv64.c:1-3000`).
- `c2mir/riscv64/`: C frontend target headers and ABI logic for RV64 Linux (`c2mir/riscv64/criscv64.h:1-55`, `c2mir/riscv64/mirc_riscv64_linux.h:5-120`, `c2mir/riscv64/criscv64-ABI-code.c:1-120`).

Fact: `mir-riscv64.c` documents an RV64 C ABI model: stack alignment, `long double` as 128-bit, `va_list` as pointer-like, aggregate passing/return rules, floating varargs through integer registers, and 17+-byte aggregate by-reference behavior (`mir-riscv64.c:7-29`).

## Likely Reusable Parts

Fact: RISC-V integer and floating register numbering is the same width-independent family shape: 32 integer registers and 32 floating registers are enumerated in `mir-riscv64.h:12-32`. Hard-register names are also family-generic (`mir-riscv64.h:35-41`).

Likely reusable with revision:

- Register identity and ABI aliases such as `ZERO`, `RA`, `SP`, `A0`-`A7`, `T*`, `S*`, `FA*`, and `FT*` (`mir-riscv64.h:12-32`).
- Some instruction encoding helpers for branch/jump immediates (`mir-riscv64.h:73-78`, `mir-gen-riscv64.c:2668-2674`).
- The pattern-table architecture: match MIR instruction forms to replacement strings, then emit fields through `out_insn` (`mir-gen-riscv64.c:1270-1397`, `mir-gen-riscv64.c:2159-2653`).
- Control-flow label reference handling and branch relaxation concepts (`mir-gen-riscv64.c:2763-2800`, `mir-gen-riscv64.c:2869-2916`).
- Backend hook structure: `target_machinize`, `target_make_prolog_epilog`, `target_split_insns`, `target_translate`, `target_rebase`, `target_bb_*`, `target_init`, and `target_finish` (`mir-gen-riscv64.c:794`, `mir-gen-riscv64.c:1092`, `mir-gen-riscv64.c:2692`, `mir-gen-riscv64.c:2736`, `mir-gen-riscv64.c:2811`, `mir-gen-riscv64.c:2827-3000`).

Inference: AArch64 is a useful structural comparison because it uses a similar hook/pattern organization and target context (`mir-gen-aarch64.c:700-753`, `mir-gen-aarch64.c:2159-2315`), but it is also a 64-bit target. It is a cleaner reference for "how another backend is organized," not for RISC-V32 data sizes.

## XLEN-Sensitive Parts

Fact: Many RV64 code paths assume 8-byte words, 64-bit integer temporaries, and 64-bit pointer storage.

Examples:

- `mir-riscv64.c` varargs use `struct riscv64_va_list { uint64_t *arg_area; }` (`mir-riscv64.c:72-75`).
- `va_arg_builtin` advances `arg_area` by 64-bit slots, with 16-byte alignment for 128-bit long double (`mir-riscv64.c:77-91`).
- `va_block_arg_builtin` advances by `sizeof(uint64_t)` slots and treats larger blocks as pointer-sized references (`mir-riscv64.c:94-106`).
- Thunk fallback for far jumps emits `ld` and stores `void *` in two instruction words as an 8-byte address (`mir-riscv64.c:125-153`, `mir-riscv64.c:167-169`).
- Wrapper code embeds `ctx`, `called_func`, and hook pointers using `sizeof(pointer)` but loads them with RV64 `ld` instructions (`mir-riscv64.c:852-881`, `mir-riscv64.c:906-930`).
- BB thunks store `bb_version` at a fixed 8-byte slot and load it with `ld` (`mir-riscv64.c:1092-1110`).
- Generator lowering creates many temporaries as `MIR_T_I64` and uses `MIR_T_I64` memory for stack/aggregate slots (`mir-gen-riscv64.c:180-240`, `mir-gen-riscv64.c:794-893`).
- Stack slots are computed as `slot * 8 + offset` (`mir-gen-riscv64.c:776-782`).
- Pattern definitions include RV64 loads/stores (`ld`, `sd`) and RV64 word operations (`addw`, `mulw`, etc.) (`mir-gen-riscv64.c:1401-1505`).
- Absolute constants and address pools use `uint64_t`, `put_uint64`, and 8-byte address entries (`mir-gen-riscv64.c:737-758`, `mir-gen-riscv64.c:2491-2504`, `mir-gen-riscv64.c:2676-2689`, `mir-gen-riscv64.c:2797-2800`).
- Rebase computes pointer relocations from 8-byte encoded values (`mir-gen-riscv64.c:2811-2822`, `mir-gen-riscv64.c:2918-2938`).

Fact: MIR itself has a pointer-size abstraction. `MIR_T_P` exists as the pointer type (`mir.h:165-175`), and `MIR_PTR32` / `MIR_PTR64` are selected from `UINTPTR_MAX` (`mir.h:182-190`). The RV64 backend has small pattern-matching checks for `MIR_PTR32` vs `MIR_PTR64` memory types (`mir-gen-riscv64.c:1938-1962`), but the surrounding backend is still RV64-only.

Inference: RISC-V32 would not just change pointer type matching. It would require replacing RV64 load/store/address-pool/thunk/ABI assumptions throughout the runtime and generator backend.

## ABI-Sensitive Parts

Fact: RISC-V64 C ABI assumptions are embedded in both runtime support and target machinization.

Examples:

- `mir-riscv64.c` explicitly documents the ABI rules used by the runtime helpers (`mir-riscv64.c:7-29`).
- `get_arg_reg` assigns normal floating arguments to `FA0`-`FA7`, other arguments to `A0`-`A7`, aligns `MIR_T_LD` to even integer registers, and consumes two integer registers for `MIR_T_LD` (`mir-gen-riscv64.c:138-178`).
- `machinize_call` implements argument stack layout, block argument passing, vararg behavior, result registers, and type extension around C calls (`mir-gen-riscv64.c:265-520`).
- `target_machinize` maps incoming function arguments from ABI registers/stack into MIR variables and handles small aggregate save areas (`mir-gen-riscv64.c:794-893`).
- `target_make_prolog_epilog` constructs stack frame setup/teardown and saves/restores used hard registers (`mir-gen-riscv64.c:1092-1239`).
- C2MIR RV64 target headers define LP64 sizes, `__riscv_xlen 64`, 8-byte pointers, 8-byte `long`, and GNU/Linux predefined macros (`c2mir/riscv64/mirc_riscv64_linux.h:5-120`, `c2mir/riscv64/criscv64.h:9-55`).
- C2MIR RV64 ABI code treats aggregates above two 8-byte words as return-by-address and classifies small structs against 2 * 8-byte limits (`c2mir/riscv64/criscv64-ABI-code.c:14-24`).

Inference: A RISC-V32 fantasy ABI could omit most of this. A real RISC-V32 C ABI backend could not.

## Runtime-Sensitive Parts

Fact: RISC-V64 native execution uses generated thunks, wrappers, executable memory publication, and code patching.

Examples:

- `_MIR_get_thunk` publishes a maximum-size jump thunk (`mir-riscv64.c:125-133`).
- `_MIR_redirect_thunk` computes a jump encoding and patches the thunk with `_MIR_change_code` (`mir-riscv64.c:177-192`).
- `_MIR_get_wrapper` uses `_MIR_get_new_code_addr` / `_MIR_publish_code_by_addr` to make PC-relative wrapper code land at the expected address (`mir-riscv64.c:852-881`).
- `_MIR_get_wrapper_end` saves ABI-visible argument/result registers, calls the lazy-generation hook, restores registers, and jumps to the returned address (`mir-riscv64.c:891-936`).
- `_MIR_get_bb_thunk`, `_MIR_replace_bb_thunk`, and `_MIR_get_bb_wrapper` implement lazy-BB triggering and patching (`mir-riscv64.c:1092-1177`).

Runtime publication and patching are provided by `_MIR_publish_code`, `_MIR_publish_code_by_addr`, `_MIR_change_code`, and `_MIR_update_code_arr` in `mir.c:4426-4489`, with page protection and cache flushing described in `docs/08-runtime-code-cache.md`.

## Whole-Function JIT Feasibility

Fact: There is no current RISC-V32 generator include path. The existing RISC-V path rejects `__riscv_xlen != 64` (`mir-gen.c:321-329`).

A true RISC-V32 whole-function JIT would require at least:

- A new backend include path and likely new files, not just enabling `mir-gen-riscv64.c`.
- RV32 hard-register and fixed-register policy, probably similar register names but different pointer/word treatment.
- RV32 instruction patterns for pointer-sized loads/stores (`lw` / `sw`) and address materialization, while preserving 64-bit MIR arithmetic where needed.
- ABI lowering for ILP32 / ILP32F / ILP32D or a documented non-C fantasy ABI.
- Stack slot sizing rules using 4-byte pointer slots where appropriate while still representing `I64` as two words or supported RV32 pairs.
- Rewritten thunks and wrappers using 32-bit pointer loads/stores and RV32 branch/address sequences.
- Rebase/relocation logic for 4-byte pointers and any 64-bit constants that remain in literal pools.
- Decisions for `long double`, double-float ABI, varargs, aggregate passing, returns, and hidden return pointers.
- C2MIR target headers and ABI code if C frontend support is desired.

Inference: Whole-function JIT is feasible as an engineering project, but not as a narrow port switch. The current backend is RV64-specific across ABI, emission, and runtime wrapper layers.

## Lazy BB JIT Feasibility

A RISC-V32 lazy basic-block JIT would require everything needed for whole-function generation plus:

- RV32 `_MIR_get_bb_thunk`, `_MIR_replace_bb_thunk`, and `_MIR_get_bb_wrapper`.
- RV32 implementations of `target_bb_translate_start`, `target_bb_insn_translate`, `target_output_jump`, `target_bb_translate_finish`, `target_bb_rebase`, `target_setup_succ_bb_version_data`, and `target_redirect_bb_origin_branch`.
- Branch redirection that can patch existing RV32 instructions safely under the code-cache protection model.
- A defined BB entry-state convention after register allocation and prolog/epilog generation.
- Re-evaluation of absolute jump-address pools and switch tables, which are currently 8-byte aligned and store 8-byte addresses (`mir-gen-riscv64.c:2638-2652`, `mir-gen-riscv64.c:2676-2689`).

Fact: The current lazy-BB implementation has a source comment suggesting incomplete attribute/version handling (`mir-gen.c:10001`) and `get_bb_version` returns the first existing version without attribute matching (`mir-gen.c:9548-9552`). This should be resolved conceptually before cloning the feature to a new backend.

Inference: Lazy BB JIT should not be the first RISC-V32 experiment. It multiplies backend work and depends on runtime code mutation.

## Fantasy Computer Simplifications

A fantasy computer ABI could intentionally omit or simplify:

- Host C ABI compatibility.
- Varargs and `va_list`.
- `long double`.
- Block argument and aggregate C ABI classification.
- Dynamic executable memory mapping/protection.
- Lazy wrappers and BB thunks.
- Thread-safe concurrent lazy generation.
- Platform predefined C macros and C2MIR target headers.
- Full MIR type set, if the subset is explicitly documented.

Mechanism classification:

- MIR modules/functions/instruction lists/operands/labels: `essential to MIR semantics`.
- RISC-V-family instruction encoding helpers: `backend-specific engineering detail`.
- RV64 C ABI lowering: `required only for C ABI compatibility`.
- Thunks/wrappers/code cache: `required for native host execution`.
- Lazy BB versioning: `optimization convenience`.
- C2MIR RV64 headers: `C2MIR-specific`.
- Pattern-table emitter: `backend-specific engineering detail`, possibly reusable as a compact implementation technique.

The term "AST" is not appropriate for this backend work. The relevant structure is MIR IR plus generator CFG/lowering state. Textual MIR parsing directly constructs MIR IR, as documented in `docs/04-textual-mir-parser.md`.

## Recommended Smallest Next Experiment

Hypothesis: The smallest useful experiment is not a full RISC-V32 JIT. It is a documented MIR subset plus either an interpreter-only execution target or a non-host "emit RV32-like code to a buffer" prototype with no C ABI and no lazy generation.

Recommended scope:

1. Define a MIR subset: integer types, pointer model, labels, branches, calls only if intra-module, no varargs, no long double, no block args.
2. Specify a fantasy ABI: register set, stack model, call/return convention, and memory layout.
3. Implement or document a small interpreter or offline emitter over the subset.
4. Only after that, compare against `mir-gen-riscv64.c` for reusable pattern-emission ideas.

This preserves MIR first: it documents the existing system before extracting a smaller design.

## Limitations And Trade-Offs

- These notes are feasibility notes, not a backend design.
- The RISC-V64 backend was inspected selectively; a complete port audit would require every pattern, target hook, builtin, and wrapper path.
- AArch64 is only a structural comparison because it is also 64-bit and ABI-heavy.
- RISC-V32 C ABI support and fantasy-computer support are different goals. Combining them early would make the smallest experiment much larger.
- Omitting lazy BB generation would remove an interesting MIR feature but greatly reduce backend/runtime complexity.

## Relevance To RISC-V32 / Fantasy Computer Extraction

RISC-V32 as a true MIR backend is the most compatible path but requires real backend work across ABI lowering, code emission, runtime thunks, code cache patching, and C2MIR integration.

RISC-V32 as a MIR-subset backend is more plausible for a constrained runtime if the subset excludes C ABI features and lazy BB generation.

An interpreter target is the lowest-risk way to validate MIR subset semantics because it avoids executable memory, relocation, instruction cache, and host ABI wrappers.

A new fantasy bytecode inspired by MIR may be smallest, but it should be clearly separated from archival MIR documentation. That would be a redesign, not preservation documentation.

## Open Questions

- Which RISC-V32 ABI would matter: ILP32, ILP32F, ILP32D, or a non-C fantasy ABI?
- Should `MIR_T_I64` be supported natively as paired RV32 registers, lowered to helper calls, or excluded from the first subset?
- Should pointer type `MIR_T_P` be distinct from `I32` in a fantasy ABI?
- Is C2MIR support required for the first RISC-V32 experiment?
- Is lazy BB generation worth preserving before whole-function generation works?
- How much of the pattern-table emitter can be reused after removing RV64-specific load/store/address-pool assumptions?
- What instruction-cache and executable-memory model does the target fantasy computer actually have?
- Should external C symbols be allowed, or should all callable symbols be MIR/fantasy-runtime internal?
- Are property instructions (`MIR_PRSET`, `MIR_PRBEQ`, `MIR_PRBNE`) required by intended workloads?
- What tests should define semantic compatibility for a MIR subset?
