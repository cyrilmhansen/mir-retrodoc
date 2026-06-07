# Code Replacement Model

## Replacement Goals

Hypothesis: A baseline compiled runtime can treat JIT as a replacement mechanism: initial baseline code runs first, then selected functions or regions can be replaced with optimized or instrumented code.

Replacement goals:

- improve performance after observation;
- add or remove instrumentation;
- isolate diagnostics;
- patch known slow paths;
- fallback to baseline or interpreter/oracle mode;
- keep replacement explainable to tools.

This is not necessarily a classic trace JIT. Tracing provides evidence and observability; replacement is a controlled runtime action.

## Replacement Units: Function, Region, Block, Trace

Function replacement:

- Inference: easiest to explain because MIR already has function items, function thunks, and whole-function generation.
- Fact: Whole-function generation redirects `func_item->addr` to `func_item->u.func->call_addr` (`mir-gen.c:9474-9485`).

Region replacement:

- Hypothesis: useful for replacing a loop or hot subgraph while preserving a baseline function.
- Question: MIR does not retain a runtime CFG after generation, so persistent region identity would need new metadata.

Block replacement:

- Fact: MIR lazy BB generation emits block versions and patches thunks/branches (`mir-gen.c:9807-9994`).
- Inference: This proves MIR can patch block-level targets, but the current mechanism is tied to lazy BBV and target-specific thunks.

Trace replacement:

- Hypothesis: not the default model for these notes.
- Question: If trace-like regions are ever used, how will they remain explainable and reversible?

## First-Prototype Replacement Scope

Hypothesis: MIR-F0 should not require region, block, or trace replacement.

Hypothesis: MIR-F0 may support no replacement at all. If replacement is included in a later first experiment, function-level replacement is the only recommended first unit.

Inference: Region, block, and trace replacement remain research notes. Lazy BBV is a source of inspiration, not a MIR-F0 dependency.

## Existing MIR Mechanisms To Inspect

Fact: Existing mechanisms relevant to replacement include:

- function thunks from module loading (`mir.c:1915-1935`);
- `_MIR_redirect_thunk` target hooks (`mir.h:721`);
- wrappers from `_MIR_get_wrapper` and `_MIR_get_wrapper_end` (`mir.h:723-724`);
- code publication through `_MIR_publish_code` (`mir.c:4426-4434`);
- patching through `_MIR_change_code`, `_MIR_update_code_arr`, and `_MIR_update_code` (`mir.c:4446-4483`);
- lazy function wrapper installation (`mir-gen.c:9779-9785`);
- lazy BB thunk creation/replacement (`mir-gen.c:9543-9566`, `mir-gen.c:9980-9994`);
- RISC-V64 branch and absolute-address rebasing (`mir-gen-riscv64.c:2811-2822`, `mir-gen-riscv64.c:2918-2967`).

Inference: MIR has strong low-level replacement primitives but not a high-level replacement policy API.

## Call-Address And Thunk Implications

Fact: MIR distinguishes the public function thunk address `func_item->addr`, the current callable target `func_item->u.func->call_addr`, and the raw generated body `func_item->u.func->machine_code` (`mir-gen.c:9474-9502`).

Inference: A future replacement model should preserve this distinction:

- stable entry address for external references;
- current active target for calls;
- archived code body records for introspection and fallback;
- replacement generation metadata.

Hypothesis: Function-level replacement should redirect a stable thunk or call slot rather than rewriting all callers initially. Direct-call rewriting can be a later optimization.

## Safety And Consistency Rules

Hypothesis: Replacement should require:

- code body fully generated and published before redirection;
- instruction cache synchronized;
- no active frames in code being removed unless replacement is tail-safe or frame-compatible;
- compatible call signature and stack/state convention;
- counters and metadata transferred or versioned;
- atomic or externally synchronized thunk/call-slot update;
- clear fallback target.

Fact: MIR's whole-function generation comments that storing `machine_code` should use atomic behavior, but atomics are not implemented in C2MIR (`mir-gen.c:9500-9501`). Lazy BB target code also contains thread-safety uncertainty around branch patching (`mir-gen-riscv64.c:2869-2916`).

Inference: Any future replacement architecture needs stricter concurrency rules than the currently documented MIR path exposes.

## Deoptimization And Active Frames

Inference: Whole-function replacement at safe points may not require full deoptimization if active frames continue running old code.

Hypothesis: Replacing code with active frames requires frame compatibility, safe points, or deoptimization.

Hypothesis: Speculative optimized regions usually require a deoptimization model if assumptions can fail.

Rule: MIR-F0 should not require deoptimization. MIR-F0 should either forbid replacement while frames are active, allow old code to continue until return, or use stop-the-world replacement only at known safe points.

Recommendation: For the first prototype, support no replacement or function-level replacement only, keep old code until module/context teardown, and do not patch active frames.

## Reversibility / Fallback

Hypothesis: Replacement should be reversible by retaining:

- baseline code address;
- replacement code address;
- reason for replacement;
- validation status;
- deoptimization/fallback path if needed;
- interpreter/oracle entry if compiled code is suspect.

Fact: MIR generated code is context-lifetime scoped and not individually freed in the inspected runtime (`mir.c:4499-4507`). This simplifies retaining old code but can increase memory use.

## Limitations And Trade-Offs

- Function replacement is simpler but less precise than region replacement.
- Region replacement needs persistent runtime block/edge metadata that MIR's generator CFG does not currently retain.
- Block replacement can be inspired by lazy BBV but inherits code mutation complexity.
- Direct-call rewriting can improve speed but makes replacement harder to explain and reverse.
- Keeping old code enables fallback but increases code-cache pressure.
- Safe replacement in concurrent execution requires atomicity or stop-the-world rules.

## Open Questions

- Question: Should the first replacement unit be whole function only?
- Question: Should old code ever be freed before context teardown?
- Question: What metadata proves a replacement is ABI/state compatible?
- Question: How should direct calls be patched or invalidated after replacement?
- Question: Can replacement be tested against interpreter oracle mode automatically?
