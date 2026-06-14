# First Prototype Contract

## Purpose

Hypothesis: This file defines a provisional first experiment, not a final architecture.

Status note: this is a historical design-perspective document. The implemented
workspace has since added `i64`, byte-memory, RV32I, optimization, JIT-demo
workflows, and partial `f32`/`f64` oracle support. The current normative status
lives in `../../ROADMAP.md` and `../experimental-rewrite/`.

Rule: MIR-F0 is not full MIR. MIR-F0 compatibility means compatibility with the explicitly documented subset only. Unsupported MIR features must not be silently accepted. A program accepted by upstream MIR is not necessarily accepted by MIR-F0. A program accepted by MIR-F0 should either be valid MIR or explicitly marked as a MIR-inspired extension.

## MIR-F0 Scope

Hypothesis: MIR-F0 is a minimal MIR-inspired subset for testing baseline compilation and observability.

Goals:

- compile or interpret all accepted functions before normal execution;
- keep the interpreter available as oracle;
- emit a trace snapshot;
- reject unsupported features explicitly;
- avoid host C ABI requirements.

## Supported Program Model

MIR-F0 should initially support:

- modules;
- functions;
- local variables/registers;
- labels/basic blocks;
- direct calls;
- returns;
- simple data/global objects if memory is included;
- no host C ABI by default.

Question: Source verification is still required to map this program model to exact upstream MIR API calls and instruction names.

## Supported Type Model

Hypothesis: Initial types should be:

- `i32`;
- `u32`;
- pointer-sized value only if memory is included;
- boolean as integer convention.

Initial feature classification:

| Feature | MIR-F0 handling | Rationale |
| --- | --- | --- |
| `i64` / `MIR_T_I64` | `reject-at-load-time` | Keeps RV32/fantasy lowering small; `lower-to-helper` can be MIR-F1 |
| floating point | `out-of-scope` | Avoids FPU and ABI questions |
| `long double` | `out-of-scope` | Avoids 128-bit/host helper complexity |
| aggregates | `out-of-scope` | Avoids C ABI aggregate rules |
| varargs | `out-of-scope` | Avoids host ABI and `va_list` rules |

Question: If memory is included, pointer representation must be finalized before implementation.

## Supported Instructions

Hypothesis: The provisional instruction subset is:

- integer constants;
- register moves;
- integer arithmetic;
- comparisons;
- load/store if memory is included;
- labels;
- unconditional branch;
- conditional branch;
- direct call;
- return.

Question: This list must be source-verified later against exact MIR instruction names in `mir.h`, `mir.c`, and `MIR.md`.

## Unsupported-Feature Handling

Unsupported features may be handled by:

- `reject-at-load-time`: invalid for the subset;
- `lower-to-helper`: compiled as a runtime helper call;
- `interpreter-only`: executable only by oracle/interpreter;
- `runtime-trap`: valid to load but traps if executed;
- `out-of-scope`: not part of the current experiment.

MIR-F0 defaults:

| Feature | Handling |
| --- | --- |
| `MIR_T_I64` | `reject-at-load-time` |
| floating-point types | `out-of-scope` |
| `long double` | `out-of-scope` |
| varargs | `out-of-scope` |
| C aggregates | `out-of-scope` |
| external C symbols | `reject-at-load-time`; explicit runtime helpers are separate |
| indirect calls | `reject-at-load-time` |
| lazy BBV | `out-of-scope` |
| direct-call rewriting | `out-of-scope` |
| function redefinition | `reject-at-load-time` |

Rule: Unsupported behavior must be reported explicitly in diagnostics and tests.

## Memory Model

Hypothesis: The safest provisional memory model is either symbolic memory or a flat 32-bit address space.

Provisional choices:

- flat 32-bit address space or symbolic memory;
- explicit alignment policy;
- trap-on-invalid-access;
- separate stack model;
- simple global data model if globals are included;
- no host pointer exposure by default.

Question: Are pointers integers, offsets, capabilities, or opaque handles?

Question: Should memory semantics follow C-like assumptions or a fantasy-machine model?

Question: MIR-F0 should define memory behavior before attempting broad backend support.

## Compilation Policy

Hypothesis: MIR-F0 should compile all loaded functions in the test module.

Rules:

- reject unresolved internal calls;
- no lazy generation as normal path;
- no silent interpreter fallback unless diagnostic mode requests it;
- interpreter remains available as oracle.

## Interpreter-Oracle Protocol

Hypothesis: The interpreter can validate baseline execution without becoming the production tier.

First-pass validation loop:

1. Load or construct a MIR-F0 program.
2. Run it through the interpreter/oracle.
3. Run it through the baseline compiler/runtime.
4. Compare return values, memory effects, traps/errors, externally visible runtime calls, trace snapshot shape, and deterministic function/block execution counts.
5. Record unsupported instructions explicitly.
6. Treat interpreter/compiler disagreement as a test failure unless unsupported behavior is documented.

## Trace Snapshot Format

Hypothesis: MIR-F0 should emit the `mir-f0-trace-snapshot-v0` schema defined in `02-runtime-introspection-and-tracing.md`.

Rule: Addresses may be symbolic or unavailable in emulator/fantasy runtimes. The schema is a prototype artifact, not an upstream MIR API.

## Replacement Policy

Rule: No replacement is required in the MIR-F0 baseline prototype.

Hypothesis: Function-level replacement can be considered for MIR-F1 or MIR-F2 after baseline execution, oracle validation, and trace snapshots work.

Rule: MIR-F0 has no region replacement, block replacement, trace replacement, deoptimization, or concurrent patching.

## Failure Model

MIR-F0 failure classes:

- validation error: reject the module;
- unsupported feature: reject at load time unless classified otherwise;
- unresolved symbol: reject unresolved internal symbols;
- backend lowering failure: reject the module or abort prototype run with diagnostics;
- register allocation failure: reject the module or abort prototype run with diagnostics;
- code allocation failure: abort the run with diagnostics;
- code publication/protection failure: abort the run with diagnostics;
- runtime patching failure: out of scope;
- interpreter/compiler mismatch: test failure.

Rule: Silent fallback is not allowed unless a diagnostic mode explicitly requests it.

## Non-Goals

- no C2MIR;
- no full RISC-V32 backend;
- no host C ABI;
- no varargs;
- no lazy BBV;
- no region replacement;
- no deoptimization;
- no concurrent patching;
- no strict W^X requirements beyond documenting the issue.

## Initial Test Programs

Initial tests:

- constant return;
- integer arithmetic;
- branch;
- loop;
- direct call chain;
- recursive or iterative Fibonacci;
- array sum if memory is included;
- simple global data access if globals are included.

## Exit Criteria

A first experiment is successful if:

- all MIR-F0 programs are accepted or rejected deterministically;
- interpreter and baseline execution agree;
- trace snapshots are emitted;
- unsupported features fail explicitly;
- documentation records all deviations from full MIR.

## Open Questions

- Question: What is the final memory model?
- Question: Should `i64` remain rejected or move to helper calls in MIR-F1?
- Question: What is the pointer representation?
- Question: What exact upstream MIR instruction names map to the MIR-F0 instruction subset?
- Question: Should the first executable target be an emulator, fantasy bytecode, or real RV32I?
