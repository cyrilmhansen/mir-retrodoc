# Design Perspective Scope

## Purpose

Fact: These documents are exploratory design notes. They are not a retrospective specification of MIR and they are not claims about Vladimir Makarov's original design intent.

The purpose is to evaluate MIR mechanisms as possible inspiration for a future baseline compiled runtime with runtime introspection, performance tracing, optional code replacement, and possible RISC-V32 or fantasy-computer targets.

Hypothesis: MIR is useful to study because it already has a compact IR, public construction APIs, textual input, an interpreter, whole-function generation, lazy generation interfaces, backend hooks, thunks, wrappers, and a code cache. Those mechanisms can be analyzed without assuming they should all be copied.

## Relationship To Main Documentation

Fact: The main documentation under `docs/` is preservation-first. It records observed architecture, source paths, symbol names, limitations, and open questions.

Inference: These design-perspective notes should depend on the main documentation, not replace it. If a design note conflicts with an observed MIR fact, the retrospective documentation remains the source of truth.

Rule: MIR-as-archived and MIR-inspired future design must remain separate. A future system may use MIR ideas while rejecting MIR mechanisms that are too tied to host ABIs, lazy generation, or OS executable-memory management.

## Source Revision And Line References

Fact: The source checkout used for the current design-perspective line references is `99c65079038f3ba9242ef646f308c266cfd7a8e5` in `mir-preservation/git/mir-restored`.

Fact: All source line references in these notes are valid only for that checkout.

Inference: Line numbers may drift after rebases, restored archives, generated-source changes, or upstream changes. Future source-grounded claims should include file path, symbol name, and commit SHA, not only line numbers.

Question: The main retrospective docs should be audited against the same commit anchor so their citations are equally reproducible.

## Licensing And Attribution Boundary

Fact: Original MIR code remains under its original license.

Fact: MIR mechanisms and observed source behavior should be attributed to the upstream MIR project and author.

Rule: Future MIR-inspired subset designs must not be presented as upstream MIR.

Inference: Documentation may analyze upstream mechanisms, but it must distinguish original behavior from new design choices. This is an engineering attribution boundary, not legal advice.

## Provisional Names

These names are provisional design-document handles. They must not imply upstream MIR compatibility.

- `MIR-Preserved`: upstream MIR behavior as documented from source.
- `MIR-Inspired`: future design family borrowing selected MIR ideas.
- `MIR-F0`: minimal fantasy/runtime subset for first experiments.
- `MIR-F1`: practical subset with more types, helper calls, and limited runtime interfaces.
- `MIR-F2`: introspective/replacement subset with counters, code versions, and controlled replacement.

Rule: MIR-F0 is not full MIR. MIR-F0 compatibility means compatibility with the explicitly documented subset only. Unsupported MIR features must not be silently accepted. A program accepted by upstream MIR is not necessarily accepted by MIR-F0. A program accepted by MIR-F0 should either be valid MIR or explicitly marked as a MIR-inspired extension.

## What This Perspective Is Allowed To Do

- Fact: Cite existing MIR mechanisms by file path and symbol name.
- Inference: Classify mechanisms by likely usefulness for baseline compilation, introspection, tracing, replacement, C ABI compatibility, host executable memory, or fantasy-computer extraction.
- Hypothesis: Propose possible future runtime models, as long as they are marked as future design directions.
- Question: Identify source areas requiring later verification.
- Hypothesis: Contrast a baseline compiled runtime with interpreter-first tracing JITs, whole-function JITs, and MIR's lazy basic-block generation.

## What This Perspective Must Not Do

- It must not rewrite MIR's retrospective documentation around this architecture.
- It must not imply MIR already implements this architecture.
- It must not present speculative runtime tracing or code replacement ideas as MIR facts.
- It must not assume interpreter-first execution, hot-loop-only compilation, or tracing as only a JIT trigger.
- It must not assume full host C ABI compatibility is core to the future design.
- It must not treat `MIR_set_lazy_bb_gen_interface` as automatically the desired architecture.
- It must not implement RISC-V32 or modify MIR source.

## Vocabulary

- `baseline compiled runtime`: A runtime where most or all valid code is compiled before normal execution.
- `load-time compilation`: Compilation performed while loading/finalizing modules, before ordinary calls run.
- `runtime introspection`: Runtime ability to expose execution state and behavior to tools or developers.
- `performance tracing`: Collection of counts, timings, frequencies, allocation behavior, and code-cache behavior.
- `code replacement`: Controlled redirection from one compiled body or region to another.
- `region recompilation`: Recompilation of a function subset, block group, or trace-like region after observation.
- `observable runtime`: Runtime designed to explain what it compiled, executed, patched, and measured.
- `interpreter as oracle`: Interpreter used as reference, diagnostic, fallback, portability path, or differential testing tool.
- `JIT as replacement mechanism`: JIT used to replace baseline code with better or instrumented code, rather than only to compile hot code.

## Open Questions

- Question: Which MIR mechanisms are essential enough to preserve in a MIR-inspired system?
- Question: Which mechanisms are host convenience rather than semantic necessity?
- Question: What minimum trace data would justify code replacement without turning the design into a conventional trace JIT?
- Question: How small can a useful baseline compiler be if host C ABI support is deferred?
- Question: Should a fantasy target preserve MIR syntax, MIR IR, or only selected MIR ideas?
