# RISC-V32 Fantasy Computer Angle

## Controlled ABI Assumptions

Hypothesis: A controlled ABI makes the smallest future runtime much easier.

If the runtime owns the ABI, it can choose:

- fixed argument registers;
- simple return convention;
- explicit stack layout;
- no host varargs;
- no `long double`;
- no C aggregate passing rules;
- no external C symbols by default;
- no hidden return pointers unless deliberately added.

Fact: MIR's current RISC-V64 support embeds a real ABI model in `mir-riscv64.c:7-29`, `mir-gen-riscv64.c:138-178`, `mir-gen-riscv64.c:265-520`, and `mir-gen-riscv64.c:794-1239`.

Inference: Avoiding host C ABI compatibility removes a large amount of porting work.

## RISC-V32-Specific Concerns

Fact: Current RISC-V generation rejects non-64-bit RISC-V (`mir-gen.c:321-329`).

Concerns:

- pointer width changes from 64 to 32 bits;
- RV64 `ld` / `sd` address handling must become RV32 pointer handling;
- 64-bit integer operations need paired-register lowering, helper calls, or exclusion;
- address pools and switch tables currently store 8-byte addresses in RV64 (`mir-gen-riscv64.c:2638-2652`, `mir-gen-riscv64.c:2676-2689`);
- thunks and wrappers load embedded pointers with RV64 `ld` instructions (`mir-riscv64.c:852-937`, `mir-riscv64.c:1092-1177`);
- C2MIR RV64 headers define LP64 sizes, `__riscv_xlen 64`, and 8-byte pointers (`c2mir/riscv64/mirc_riscv64_linux.h:5-120`, `c2mir/riscv64/criscv64.h:9-55`).

Question: Should `MIR_T_I64` be part of the first RISC-V32 subset, or should it be deferred to helper calls?

## Unsupported-Feature Taxonomy

Unsupported features may be handled by:

- `reject-at-load-time`: invalid for the subset;
- `lower-to-helper`: compiled as a runtime helper call;
- `interpreter-only`: executable only by oracle/interpreter;
- `runtime-trap`: valid to load but traps if executed;
- `out-of-scope`: not part of the current experiment.

Hypothesis: MIR-F0 classification:

| Feature | MIR-F0 handling |
| --- | --- |
| `MIR_T_I64` | `reject-at-load-time` initially; `lower-to-helper` is a MIR-F1 candidate |
| floating-point types | `out-of-scope` |
| `long double` | `out-of-scope` |
| varargs | `reject-at-load-time` |
| C aggregates | `reject-at-load-time` |
| external C symbols | `reject-at-load-time`; explicit runtime helpers may be declared separately |
| indirect calls | `reject-at-load-time` |
| lazy BBV | `out-of-scope` |
| direct-call rewriting | `out-of-scope` |
| function redefinition | `reject-at-load-time` |

Rule: Unsupported MIR features must not be silently accepted by MIR-F0.

## Memory And Object Model Questions

Question: Should MIR-F0 use flat memory or segmented memory?

Question: Is pointer width fixed at 32 bits, symbolic, or target-dependent?

Question: What alignment is required for loads, stores, stack slots, and globals?

Question: Is the runtime little-endian, big-endian, or explicitly parameterized?

Question: Do invalid accesses trap deterministically?

Question: What is the stack model, and can stack addresses escape?

Question: What is the global data model?

Question: Where is the heap/allocation boundary?

Question: Are pointers integers, offsets, capabilities, or opaque handles?

Question: Do memory semantics follow C-like assumptions or a fantasy-machine model?

Question: A first MIR-F0 prototype should define memory behavior before attempting broad backend support.

## Fantasy Computer Simplifications

Hypothesis: A fantasy computer can simplify more aggressively than a real RISC-V32 host backend.

Possible simplifications:

- compile at load time into a fixed memory arena;
- use a fantasy ABI, not host C ABI;
- avoid dynamic linking to C symbols;
- omit lazy BBV;
- expose code ranges and counters through emulator/runtime metadata;
- treat interpreter as oracle, not first-tier execution;
- define a reduced type set;
- make replacement explicit and stop-the-world.

Inference: These choices make the runtime easier to explain and test, but reduce compatibility with full MIR/C2MIR.

## Existing RISC-V64 Backend Relevance

Useful inspiration:

- register naming and ABI aliases in `mir-riscv64.h:12-41`;
- immediate encoding helpers such as `get_j_format_imm` (`mir-riscv64.h:73-78`);
- pattern-table approach in `mir-gen-riscv64.c:1270-1397`;
- target hook structure in `mir-gen-riscv64.c:2736-3000`;
- branch/reference rebasing concepts in `mir-gen-riscv64.c:2763-2822`.

Probably not directly reusable:

- RV64 thunks/wrappers using 8-byte embedded pointers and `ld` (`mir-riscv64.c:125-192`, `mir-riscv64.c:852-937`);
- RV64 varargs and aggregate C ABI support (`mir-riscv64.c:72-117`, `mir-gen-riscv64.c:265-520`);
- RV64 long-double helper expectations (`mir-gen-riscv64.c:507-714`);
- C2MIR RV64 predefined headers (`c2mir/riscv64/*`).

## First Prototype Options

Real RISC-V32 machine code:

- Inference: highest compatibility with future hardware, highest implementation cost.

Emulator target:

- Hypothesis: good balance if the emulator can expose counters, code ranges, and traps without OS executable-memory complexity.

Fantasy bytecode:

- Hypothesis: smallest runtime surface, but furthest from MIR as archived.

Simplified MIR interpreter:

- Hypothesis: best first semantic experiment if the goal is to validate a MIR subset before code generation.

## Recommended Smallest Experiment

Hypothesis: Define a small MIR subset and run it through a baseline compiled or interpreted fantasy target without host C ABI support.

Minimum experiment:

1. Select a subset: integer arithmetic, local variables, labels, branches, simple calls, no varargs, no `long double`, no C aggregates.
2. Define a fantasy ABI and code metadata schema.
3. Compile or interpret every function at load time.
4. Collect function and block counters.
5. Emit a trace snapshot explaining what executed.

This tests baseline compilation and observability without rewriting MIR or attempting a full RISC-V32 backend.

## Open Questions

- Question: Is the first target real RV32I/RV32IM/RV32IMF/RV32IMFD or a fantasy instruction set?
- Question: Is binary compatibility with C ever required?
- Question: What is the smallest useful MIR type subset?
- Question: Should pointer operations be explicit fantasy operations rather than C-like pointers?
- Question: How should the subset handle memory and allocation?
