# Lazy Basic-Block JIT

## Overview

Lazy basic-block generation is exposed through `MIR_set_lazy_bb_gen_interface`, declared in `mir-gen.h:26` and implemented in `mir-gen.c:10007-10012`.

The MIR manual describes the mode as generating machine code for function basic blocks only on their first execution (`MIR.md:713-715`). Property instructions are specifically documented as having no ordinary machine instruction and being useful for specialized code when lazy basic-block versioning is used (`MIR.md:543-548`).

Observed implementation units:

- `struct bb_stub`: identifies a function and an instruction range for one basic block (`mir-gen.c:351-355`).
- `struct bb_version`: identifies a compiled or compilable version of a `bb_stub`, with an attribute vector, call-entry flag, thunk/address, machine-code address, and target-specific data (`mir-gen.c:337-345`).
- `generate_bb_version_machine_code`: emits one basic-block version (`mir-gen.c:9807-9994`).
- `bb_version_generator`: wrapper hook that calls `generate_bb_version_machine_code` and returns the generated address (`mir-gen.c:9996-9999`).

## Relationship To Whole-Function JIT

Lazy BB mode reuses much of the whole-function pipeline, but it does not publish a whole-function body.

First function call path:

1. `MIR_set_lazy_bb_gen_interface` installs a wrapper for the function thunk (`mir-gen.c:10007-10012`).
2. The wrapper calls `generate_func_and_redirect_to_bb_gen` (`mir-gen.c:10001-10004`).
3. That calls `generate_func_and_redirect(ctx, func_item, FALSE)` (`mir-gen.c:10001-10004`).
4. `generate_func_and_redirect(..., FALSE)` runs `generate_func_code(..., FALSE)`, then calls `create_bb_stubs`, creates the entry BB version, and redirects the function thunk to that entry version address/thunk (`mir-gen.c:9763-9771`).

In `generate_func_code(..., FALSE)`, the pipeline still duplicates instructions, builds a CFG, lowers, allocates registers, builds prolog/epilog, and splits target instructions. It skips whole-function `target_translate`, `_MIR_publish_code`, `target_rebase`, function-thunk redirection, original-instruction restoration, and `machine_code` assignment (`mir-gen.c:9277-9502`).

Because `_MIR_restore_func_insns` is only called when `machine_code_p` is true (`mir-gen.c:9490-9501`), lazy BB mode appears to keep the transformed duplicated instruction stream for later block-version emission. The temporary CFG itself is destroyed before returning from `generate_func_code` (`mir-gen.c:9487-9490`, `mir-gen.c:1889-1912`).

## Basic-Block Version Concept

A `bb_stub` is a block descriptor over an instruction range:

- `func_item`: owning MIR function.
- `first_insn`: first instruction in the block.
- `last_insn`: last instruction in the block.
- `bb_versions`: list of versions for that block (`mir-gen.c:351-355`).

A `bb_version` is a compiled-or-pending version of one `bb_stub`:

- `call_p`: marks versions entered as a function call entry rather than an internal block successor.
- `addr`: initially a thunk address, later the generated BB code address.
- `machine_code`: generated code address after emission.
- `target_data`: backend-specific version data used for successor patching/origin branch redirection.
- `attrs`: property attributes associated with the version (`mir-gen.c:337-345`).

`get_bb_version` currently returns the first existing version for a stub without comparing requested attributes if any version already exists (`mir-gen.c:9543-9552`). It allocates a new version only when the version list is empty, creates a target version data object, records attributes, creates a BB thunk with `_MIR_get_bb_thunk`, and returns its address (`mir-gen.c:9553-9566`).

The source comment `attrs ignored ??? implement versions` appears near the lazy-BB entry hook (`mir-gen.c:10001`). This suggests version attribute handling may be incomplete. Treat the full "versioning" design as partially implemented unless later inspection proves otherwise.

## Trigger Mechanism

Two lazy triggers are present:

- Function-level trigger: the function thunk initially jumps to a wrapper created by `_MIR_get_wrapper`. On first function call, this wrapper runs `generate_func_and_redirect_to_bb_gen` (`mir-gen.c:10001-10012`).
- Basic-block trigger: new BB versions initially receive a target-specific BB thunk from `_MIR_get_bb_thunk(ctx, bb_version, bb_wrapper)` (`mir-gen.c:9564-9566`). The thunk calls the shared `bb_wrapper`, which invokes `bb_version_generator`; that generator emits the block and returns the generated address (`mir-gen.c:9703`, `mir-gen.c:9996-9999`).

On x86-64, `_MIR_get_bb_thunk` builds a small thunk that loads the `bb_version` into `r10` and jumps to the handler (`mir-x86_64.c:826-838`). `_MIR_get_bb_wrapper` saves registers, calls the hook, restores registers, and jumps to the returned address (`mir-x86_64.c:918-971`).

On RISC-V64, `_MIR_get_bb_thunk` stores the `bb_version` in generated code and redirects to the handler; `_MIR_get_bb_wrapper` saves/restores registers and jumps through `t5` (`mir-riscv64.c:1092-1126`, `mir-riscv64.c:1126-1165`).

## Block Boundary And State Model

`create_bb_stubs` scans the transformed function instruction list and starts a new block at:

- the function entry or after a previous terminating instruction;
- a `MIR_LABEL`;
- the instruction following any branch, `MIR_RET`, `MIR_JRET`, `MIR_PRBEQ`, or `MIR_PRBNE` (`mir-gen.c:9571-9616`).

Consecutive labels are grouped into the same block, and label instruction `data` fields are set to the corresponding `bb_stub` (`mir-gen.c:9581-9588`, `mir-gen.c:9603-9609`). Label-reference data are then patched to point at BB version thunks, with label-difference handling when two labels are involved (`mir-gen.c:9627-9638`).

Runtime state between blocks appears to rely on the normal generated-code machine state after register allocation and prolog/epilog formation. Successor transfer addresses are passed to target translation hooks through `succ_bb_addrs`, and target-specific version data are passed through `target_succ_bb_versions` (`mir-gen.c:244-248`, `mir-gen.c:9926-9932`, `mir-gen.c:9952-9979`).

Open point: the exact block-entry ABI is backend-specific and should be documented by inspecting each target's `target_bb_*` hooks.

## Thunks And Patching

When a new BB version is created, its `addr` is initially a generated thunk (`mir-gen.c:9564-9566`). After `generate_bb_version_machine_code` emits real code:

- `target_bb_translate_finish` returns code bytes (`mir-gen.c:9980`).
- `_MIR_publish_code` publishes the BB code (`mir-gen.c:9980-9982`).
- `target_bb_rebase` rebases target references (`mir-gen.c:9981-9982`).
- `target_setup_succ_bb_version_data` records successor data (`mir-gen.c:9983`).
- `target_redirect_bb_origin_branch` can redirect an origin branch to the new address (`mir-gen.c:9989`).
- `_MIR_replace_bb_thunk(ctx, bb_version->addr, addr)` patches the original thunk to jump directly to generated code (`mir-gen.c:9989-9991`).
- `bb_version->addr` and `bb_version->machine_code` are set to the generated address (`mir-gen.c:9990-9993`).

On x86-64, `_MIR_replace_bb_thunk` overwrites the thunk with a relative jump by calling `_MIR_change_code` twice (`mir-x86_64.c:841-848`). On RISC-V64, it delegates to `redirect_thunk` (`mir-riscv64.c:1119-1122`).

These operations depend on executable code mutation. `_MIR_change_code` writes into existing code memory using `_MIR_set_code` and flushes the instruction cache (`mir.c:4446-4457`). `_MIR_set_code` temporarily marks memory write/execute and then read/execute (`mir.c:4398-4409`).

## Runtime Code Replacement

Lazy BB code replacement is local to generated thunks and branch redirection:

- First entry to a BB version goes through the version thunk and generator wrapper.
- After generation, the thunk is replaced with a direct jump to the generated BB code.
- Target-specific hooks can redirect predecessor/origin branches to the generated block code.
- Later entries should avoid the generator wrapper for already-patched versions.

This is not trace recording. The inspected code does not record interpreter execution traces, profile hot paths, or compile arbitrary dynamic traces. It partitions a transformed function into static basic blocks and lazily emits code for block versions as execution reaches them.

## Property Instructions

The relevant MIR opcodes are declared in the instruction descriptor table:

- `MIR_PRSET`: operands are an undefined-kind target and integer property (`mir.c:335`).
- `MIR_PRBEQ`: label, undefined-kind target, integer property (`mir.c:336`).
- `MIR_PRBNE`: label, undefined-kind target, integer property (`mir.c:337`).

The interpreter mostly treats properties as unknown/default-zero: `MIR_PRSET` is ignored; `MIR_PRBEQ` jumps only for property zero; `MIR_PRBNE` jumps for nonzero property constants (`mir-interp.c:416-424`).

In lazy BB generation, `generate_bb_version_machine_code` tracks nonzero property spots. `MIR_PRSET` updates or clears a tracked property. `MIR_PRBEQ` and `MIR_PRBNE` can be removed or converted to unconditional jumps depending on the tracked property value (`mir-gen.c:9843-9888`). Moves can propagate or clear property information, including coarse handling of memory aliases (`mir-gen.c:9889-9915`).

Classification:

- Property instructions: `optimization convenience`.
- Default interpreter behavior for property instructions: `essential to preserving execution if property instructions appear`, but not essential to core MIR arithmetic/control-flow semantics.
- Memory property alias handling: `backend-independent optimization engineering detail`.

## Limitations And Trade-Offs

- Lazy BB mode is optional and substantially more complex than whole-function generation.
- The source contains an explicit comment indicating attributes are ignored or versioning is unfinished (`mir-gen.c:10001`), and `get_bb_version` returns the first existing version without matching requested attributes (`mir-gen.c:9548-9552`).
- Block generation depends on mutable executable thunks and branch patching. This is harder on strict W^X systems or environments with expensive instruction-cache synchronization.
- The first function call still pays a preparatory whole-function pipeline cost before any block code is emitted.
- Compared with whole-function JIT: lower initial block emission cost, but more runtime patching and thunk overhead.
- Compared with trace JIT: simpler static block boundaries, but no observed hot-trace profiling or cross-block trace formation.
- Compared with method/function JIT: more incremental code generation, but more complicated control transfer and version state.
- Compared with interpreter-only execution: faster once blocks are generated, but requires native backend support and executable-memory management.

## Relevance To RISC-V32 / Fantasy Computer Extraction

- Static block partitioning over MIR instruction lists: `optimization convenience`; reusable in a smaller compiler if block-at-a-time emission is desired.
- BB version thunks/wrappers: `backend-specific engineering detail` and `required for native host execution`.
- C hook calls from wrappers: `required only for C ABI compatibility`.
- Property instruction specialization: `optimization convenience`; removable for a fantasy subset unless specialized block versions are a design goal.
- Code mutation, thunk replacement, and branch patching: `required for native host execution` in this mode, but removable for interpreter-only or ahead-of-time fantasy variants.

For a RISC-V32 backend, this mode would require new 32-bit BB thunks, wrappers, register-save conventions, branch redirection logic, target BB translation hooks, relocation/rebase support, and cache-flush behavior. The existing RISC-V backend code is RV64-only (`mir-gen.c:321-329`, `mir-riscv64.c:1092-1165`).

For a fantasy computer subset, omitting lazy BB generation would be a defensible simplification. The core MIR concepts of modules, functions, instruction lists, labels, operands, and whole-function interpretation/generation do not appear to depend on BB versioning.

## Open Questions

- Is attribute-sensitive BB versioning intentionally incomplete, or is the first-version reuse in `get_bb_version` sufficient for current C2MIR output?
- Which target hooks define the exact BB entry/exit ABI?
- How does lazy BB generation interact with functions containing calls, varargs, block arguments, or unusual control flow?
- Are thunk replacement and origin branch redirection safe under concurrent execution?
- What tests cover `MIR_set_lazy_bb_gen_interface` specifically?
