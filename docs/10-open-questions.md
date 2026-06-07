# Open Questions

This file collects project-level questions raised during the first source-map pass. Deeper documents should either answer these questions with citations or split them into more precise questions.

## IR Construction

- Does `MIR_scan_string` build any retained AST-like structure, or does it construct modules, items, labels, instructions, and operands directly?
- Which validation steps happen during API calls, during text scanning, during binary reading, during `MIR_finish_func`, and during `MIR_load_module`?
- Which user-visible MIR objects are mutated by `MIR_finish_func`?
- What are the exact ownership rules for strings, data blobs, instruction operands, and function variable arrays?
- When and why are `MIR_func_t.insns` and `MIR_func_t.original_insns` both used?
- Are labels module-scoped, function-scoped, or mixed depending on item type? Existing `MIR.md` says label scope is whole module for `lref`; verify general label handling.

## Textual MIR And Binary MIR

- What binary format versioning or magic exists, if any?
- Can binary MIR round-trip all text/API-created structures exactly?
- Which semantic checks differ between `MIR_scan_string` and `MIR_read_with_func`?
- Are binary MIR files intended as archival artifacts, executable artifacts, or only fast load artifacts?

## Interpreter

- What exact dispatch mechanism does `mir-interp.c` use?
- Which MIR instructions are unsupported, restricted, or treated specially in the interpreter?
- How are calls among interpreted MIR functions, generated MIR functions, and external C functions distinguished or unified?
- Does the interpreter serve mainly as a portability layer, correctness fallback, bootstrap path, or production execution engine?
- How much target code is required before the interpreter works on a new architecture?
- Could a fantasy-computer interpreter remove `_MIR_get_interp_shim`, `_MIR_get_ff_call`, and varargs helpers if it does not interoperate with host C?

## JIT Pipeline

- Which interfaces compile complete functions and which compile smaller basic-block versions?
- What is the exact sequence in `generate_func_code` at each optimization level?
- Which generator transformations are backend-independent, and which are delegated to target files?
- Does generated machine code correspond one-to-one with a complete MIR function for `MIR_gen` and `MIR_set_lazy_gen_interface`?
- How does lazy basic-block versioning use `MIR_PRSET`, `MIR_PRBEQ`, and `MIR_PRBNE`?
- Are generated code blocks immutable after emission except explicit patching helpers?

## Optimizations

- What does optimization level `>=3` enable beyond the documented `-O2` pipeline?
- What cost model is used for block cloning, GVN, LICM, pressure relief, coalescing, live-range splitting, and combine?
- Which passes are explicitly omitted to keep compile time low?
- Are optimization decisions based on profile/hotness data, static heuristics, or basic-block versioning attributes?
- How much of the optimizer is required before a backend can generate correct but slower code?

## Runtime And Code Cache

- Does MIR free individual generated functions or only free generated code when `MIR_finish` destroys the context?
- How are code holders and machine-code contexts organized in `mir.c`?
- What relocation and patching operations are supported by `_MIR_set_code`, `_MIR_change_code`, and `_MIR_update_code_arr`?
- What instruction-cache synchronization is required on each target?
- Are W^X assumptions fully enforced, approximated, or platform-dependent?
- What are the thread-safety assumptions for code publication, lazy generation, linking, and context finalization?

## Backend Architecture

- Which target hooks are mandatory for interpreter-only support?
- Which target hooks are mandatory for whole-function generation?
- Which target hooks are only for lazy generation or basic-block versioning?
- Which existing backend is the smallest and clearest reference model?
- Which backend is most mature according to tests, CI, and issue history?
- Is `mir-gen-stub.c` only a no-generator placeholder, or does it define a meaningful fallback interface?

## RISC-V32 / Fantasy Computer Feasibility

- What MIR semantics depend on 64-bit integer variables, and what only depends on target pointer size?
- Can RISC-V64 target support be adapted to RISC-V32 without changing core MIR semantics?
- Which C2MIR target assumptions are invalid for RISC-V32?
- Would a fantasy computer subset own its ABI enough to remove block argument ABI cases, varargs, foreign-function shims, and host executable-memory APIs?
- Which MIR instructions are essential for the intended fantasy workload?
- Can long double, multi-result returns, label-address operations, and indirect jumps be omitted from a subset without misrepresenting it as full MIR?
- Is a full native JIT necessary, or would an interpreter plus compact bytecode-like internal form satisfy the preservation-derived subset goal?

## Historical And Archival Evidence

- Which preserved web articles or slides contain design rationale not present in source comments?
- Are there GitHub issues/discussions about RISC-V, porting, 32-bit targets, Windows, Java, MicroPython, or lightweight subsets?
- Which source revision is represented by `mir-preservation/git/mir-restored`, and does it correspond to a tagged release?
- Do release notes indicate backend maturity or known limitations?

## Current First-Pass Answers To Preserve

- RISC-V64 files exist: `mir-riscv64.c`, `mir-riscv64.h`, `mir-gen-riscv64.c`, and `c2mir/riscv64/`.
- No RISC-V32 backend file was observed in the first pass.
- The main in-memory MIR representation appears to be IR objects and instruction lists, not a retained AST.
- `mir-gen.c` documents a latency/quality trade-off: `-O0` and `-O1` are described as 2-3 times faster than `-O2` but generating considerably slower code.
- The interpreter still requires target-specific code for C-callable shims, FFI calls, varargs, and stack builtins.
