# MIR Mechanism Reuse Map

## Classification Table

| Mechanism | Classification | Source anchors |
| --- | --- | --- |
| MIR context and modules | useful for baseline compilation; useful for runtime introspection | `struct MIR_context` in `mir.c:31-62`; module lifecycle in `mir.c:927-936`, `mir.c:1767-1772` |
| MIR function items | useful for baseline compilation; useful for code replacement | function generation and thunk update in `mir-gen.c:9277-9502` |
| MIR instruction lists | useful for baseline compilation; useful for introspection | function instruction lists used by parser/API/generator; duplication in `mir-gen.c:9302-9303` |
| MIR textual parser | useful for baseline compilation; probably preservable for tooling | `MIR_scan_string` direct construction path in `mir.c:6286-6800` |
| MIR binary format | unclear / requires source trace | binary IO files exist; not analyzed in this pass |
| Interpreter | useful as interpreter as oracle; useful for diagnostics | interpreter execution mode in `MIR.md:723-744`; expression data in `mir.c:2037-2058` |
| `MIR_gen` | useful for baseline compilation | `MIR_gen` in `mir-gen.c:9505-9506` |
| `MIR_set_gen_interface` | useful for load-time compilation | `mir-gen.c:9755-9761` |
| `MIR_set_lazy_gen_interface` | useful mainly for lazy JIT; possible fallback inspiration | `mir-gen.c:9779-9785` |
| `MIR_set_lazy_bb_gen_interface` | useful mainly for lazy JIT; useful as code replacement inspiration | `mir-gen.c:10007-10012` |
| CFG construction | useful for baseline compilation; useful for possible introspection if persisted | `build_func_cfg` in `mir-gen.c:1571-1887` |
| Optimization levels | useful for staged baseline/optimized compilation | level setup in `mir-gen.c:207-210`, `mir-gen.c:9648-9650`; manual in `MIR.md:763-772` |
| Register allocation | useful for native baseline compilation | `reg_alloc` call in `mir-gen.c:9443-9447` |
| Target backends | useful for native compilation; backend-specific engineering detail | target includes in `mir-gen.c:313-329` |
| Thunks | useful for code replacement; useful mainly for host execution | `_MIR_redirect_thunk` in `mir.h:721`; target implementations |
| Wrappers | useful for lazy JIT; useful mainly for C ABI compatibility | `_MIR_get_wrapper` declarations in `mir.h:723-724`; RISC-V64 wrapper in `mir-riscv64.c:852-937` |
| Code allocator | useful mainly for host executable memory management | `mir-code-alloc.h:16-38`; `mir-code-alloc-default.c:18-82` |
| Code patching helpers | useful for code replacement; host executable memory management | `_MIR_change_code`, `_MIR_update_code_arr` in `mir.c:4446-4483` |
| External symbol loading | useful mainly for C ABI compatibility | `MIR_load_external` in `mir.c:1956-1963`; import resolution in `mir.c:1995-2005` |
| C2MIR | useful for source compatibility; probably out of scope for first fantasy prototype | C2MIR target headers such as `c2mir/riscv64/mirc_riscv64_linux.h` |
| RISC-V64 backend | useful as backend reference; XLEN/ABI-specific | `mir-riscv64.c`, `mir-riscv64.h`, `mir-gen-riscv64.c` |

## Notes By Mechanism

Fact: MIR context, module, item, function, and instruction-list structures are core to how MIR represents programs.

Inference: These are worth preserving as documentation anchors even if a future subset changes execution.

Fact: `MIR_gen` and `MIR_set_gen_interface` already express whole-function generation and link-time generation.

Inference: They are stronger references for a baseline compiled runtime than lazy BBV.

Fact: The code allocator and patching helpers are host-native execution infrastructure, not MIR IR semantics.

Hypothesis: A fantasy-computer subset can replace them with a fixed code arena, emulator hooks, or no native code cache at all.

## Mechanisms Likely Worth Preserving

- MIR context/module/function/item lifecycle.
- MIR instruction-list IR model.
- Textual MIR as a tool and archival format.
- Explicit whole-function generation concept.
- Interpreter as oracle or diagnostic path.
- Basic CFG construction ideas, if persistent metadata is needed later.

## Mechanisms Likely Worth Simplifying

- Host C ABI wrappers.
- External symbol resolution.
- Varargs and block aggregate ABI rules.
- Code allocator protection model for fantasy targets.
- Lazy generation wrappers if baseline compilation is the default.

## Mechanisms Probably Out Of Scope For First Prototype

- Lazy basic-block versioning.
- Full C2MIR target support.
- Full RISC-V64 backend porting to RISC-V32.
- Thread-safe concurrent code replacement.
- Direct-call rewriting after replacement.
- Strict W^X support beyond what a prototype target requires.

## Prototype Relevance

### MIR-F0

- context/module/function lifecycle;
- instruction-list IR model;
- textual construction or API construction;
- interpreter as oracle;
- whole-function baseline compilation concept;
- function-level counters.

### MIR-F1

- helper calls;
- limited runtime traps;
- code metadata;
- limited `i64` support;
- memory model expansion.

### MIR-F2

- code replacement;
- block/edge counters;
- region recompilation;
- lazy BBV inspiration;
- persistent CFG/runtime graph.

## Open Questions

- Question: Should a future subset preserve MIR textual syntax or define a smaller syntax?
- Question: Which optimization level best represents "baseline"?
- Question: Can the generator CFG be made persistent without pulling in the full optimizer?
- Question: Which C ABI features are genuinely needed by target workloads?
- Question: Is binary MIR important for load-time compilation?
