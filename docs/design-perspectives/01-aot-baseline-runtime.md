# AOT Baseline Runtime

## Terminology Note

Inference: "AOT" is used informally in this file.

The more precise terms are `baseline-compiled runtime` and `load-time baseline compilation`. This does not necessarily mean a separate offline object-code artifact.

Fact: `MIR_set_gen_interface` is closer to load-time whole-function generation than to traditional offline AOT: it is passed to `MIR_link`, which invokes the interface for each loaded function (`mir.c:2061-2072`, `mir-gen.c:9755-9761`).

## Baseline Compilation Model

Hypothesis: A baseline compiled runtime would compile most or all valid functions before normal execution. Compilation could occur at build time, load time, before first execution, or during a controlled module-finalization phase.

Fact: MIR already supports explicit whole-function generation through `MIR_gen(ctx, func_item)`, declared in `mir-gen.h:23` and implemented as `generate_func_code(ctx, func_item, TRUE)` (`mir-gen.c:9505-9506`). Whole-function generation publishes code and redirects the function thunk (`mir-gen.c:9474-9502`).

Fact: MIR also supports link-time generated execution through `MIR_link(ctx, MIR_set_gen_interface, resolver)`. `MIR_link` calls the selected interface for each loaded function and then with `NULL` as a finish signal (`mir.c:2061-2072`). `MIR_set_gen_interface` calls `MIR_gen` for each function and then asks the target to change to direct calls on finalization (`mir-gen.c:9755-9761`).

Inference: `MIR_set_gen_interface` is the closest existing MIR mechanism to systematic load-time whole-function compilation. It is not AOT in the separate-build-artifact sense, but it does compile linked functions before ordinary execution through their call addresses.

## Possible Execution Pipeline

Hypothesis: A future baseline runtime could use this staged pipeline:

1. Parse or construct IR.
2. Finalize modules and validate functions.
3. Link symbols inside a controlled environment.
4. Compile every accepted function to baseline code.
5. Publish function metadata, code ranges, and debugging/introspection records.
6. Run normal execution from baseline code.
7. Collect performance tracing data.
8. Optionally replace functions or regions with optimized or instrumented versions.

Fact: MIR already separates construction/loading/linking from generation. Modules are loaded by `MIR_load_module`, linked by `MIR_link`, and generation interfaces can be selected during linking (`mir.c:1915-1953`, `mir.c:1969-2072`, `mir-gen.c:9755-10012`).

Question: How much of `MIR_link` simplification and inline processing would be desirable in a future baseline runtime versus preserved only for MIR compatibility?

## Compilation Coverage Policy

Hypothesis: A baseline-compiled runtime needs an explicit coverage policy before execution starts.

Options:

- compile all loaded functions;
- compile only functions reachable from exports or entry points;
- compile public/exported functions plus reachable internals;
- compile cold functions with minimal baseline;
- reject unresolved functions before compilation;
- keep unresolved functions interpreter-only.

Hypothesis: For MIR-F0, compile all loaded functions in the test module, reject unresolved internal calls, and allow only explicitly declared runtime traps/helpers.

Inference: This policy can be relaxed later for modules, libraries, plugins, or partial-load systems.

## Failure Model

Hypothesis: MIR-F0 should fail explicitly rather than silently shifting execution tiers.

Failure classes:

- validation error: reject the module;
- unsupported feature: reject at load time unless explicitly classified otherwise;
- unresolved symbol: reject unresolved internal symbols at load time;
- backend lowering failure: reject the module or abort the run in prototype tooling;
- register allocation failure: reject the module or abort the run in prototype tooling;
- code allocation failure: abort the run with a reported resource failure;
- code publication/protection failure: abort the run with a reported runtime preparation failure;
- runtime patching failure: out of scope for MIR-F0 because replacement is not required;
- interpreter/compiler mismatch: test failure.

Hypothesis: MIR-F0 should avoid silent fallback unless a diagnostic mode explicitly requests interpreter-only execution.

## Interpreter Role

Hypothesis: The interpreter would not be the normal first tier. It would serve as:

- reference implementation;
- diagnostic execution mode;
- fallback when code generation is unavailable;
- portability path;
- differential testing oracle for compiled code.

Fact: MIR has an interpreter and the manual describes interpretation as an execution mode (`MIR.md:723-744`). MIR also uses interpretation during linking for expression data (`mir.c:2037-2058`).

Inference: Even in a baseline compiled runtime, an interpreter can remain useful for constant/expression evaluation, testing, and failure isolation without becoming the primary execution tier.

## Interpreter-Oracle Validation Protocol

Hypothesis: The interpreter can validate a subset runtime without becoming the production tier.

First-pass loop:

1. Load or construct a MIR-F0 program.
2. Run it through the interpreter/oracle.
3. Run it through the baseline compiler/runtime.
4. Compare return values, memory effects, traps/errors, externally visible runtime calls, trace snapshot shape, and deterministic function/block execution counts.
5. Record unsupported instructions explicitly.
6. Treat interpreter/compiler disagreement as a test failure unless the unsupported behavior is documented.

Inference: The interpreter is a reference and diagnostic tool for the subset, not necessarily the normal runtime tier.

## Whole-Function Compilation Vs Lazy Compilation

Fact: `MIR_gen` compiles a complete MIR function (`mir-gen.c:9277-9502`). `MIR_set_lazy_gen_interface` installs a wrapper that compiles the whole function on first call (`mir-gen.c:9779-9785`). `MIR_set_lazy_bb_gen_interface` prepares lazy basic-block generation on first call and then emits block versions on first execution (`mir-gen.c:10001-10012`, `mir-gen.c:9807-9994`).

Inference: A baseline compiled runtime would favor explicit/eager whole-function generation and treat lazy generation as optional fallback or later optimization.

Hypothesis: Lazy BB generation should not be copied as the default architecture. It is valuable as evidence that MIR can patch thunks and generate smaller units, but it is tied to runtime code mutation and target-specific wrappers.

## Startup And Memory Trade-Offs

Inference: Compiling all functions before execution improves predictability and avoids first-call pauses, but can increase startup time and code memory use.

Fact: MIR's code cache is context scoped and does not expose individual generated-function freeing in the inspected runtime (`mir.c:4353-4507`). Generated code is published into page holders through `_MIR_publish_code` (`mir.c:4426-4434`).

Hypothesis: A future baseline runtime would need code-size accounting and possibly per-function metadata before systematic compilation is practical for larger programs.

Question: Should cold functions be compiled in a minimal form, compiled on demand, or interpreted by oracle mode?

## Debuggability And Determinism

Inference: Load-time compilation can make execution more deterministic than first-call lazy generation because all compilation failures occur before normal execution.

Hypothesis: A baseline runtime should record the compilation decision for every function: accepted, rejected, compiled baseline, compiled optimized, interpreted fallback, or replaced.

Fact: MIR generator debug output can be configured through `MIR_gen_set_debug_file` and `MIR_gen_set_debug_level` (`mir-gen.c:9509-9528`), but this is primarily generator debugging rather than a structured tooling interface.

Question: What stable metadata format should explain compiled functions, code ranges, and replacement events?

## Open Questions

- Question: Is `MIR_set_gen_interface` sufficient as a source model for load-time compilation, or does it mix too much linking and generation policy?
- Question: Should baseline compilation require no optimization, level 1, or MIR's default level 2?
- Question: How should compilation failure be reported without falling into undefined runtime state?
- Question: Can a future system avoid host C ABI wrappers for internal calls?
- Question: Should all functions be compiled, or only loaded/exported/reachable functions?
