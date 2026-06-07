# Risks And Non-Goals

## Architectural Risks

Risk: The design could become a second full VM architecture instead of a preservation-informed experiment.

Risk: Baseline compilation plus tracing plus replacement plus RISC-V32 plus fantasy-computer support is too much for one prototype.

Risk: Copying MIR's host ABI machinery too early could obscure the smaller idea: a controlled, observable baseline runtime.

Risk: Avoiding the host C ABI may make the design cleaner but reduce compatibility with existing MIR/C2MIR programs.

## Complexity Risks

Inference: MIR's native execution path spans IR, CFG, optimization, register allocation, target lowering, code publication, thunks, wrappers, and runtime patching. Copying all of it would be a large project.

Fact: RISC-V64 support is RV64-only and ABI-heavy (`mir-gen.c:321-329`, `mir-riscv64.c:7-29`, `mir-gen-riscv64.c:265-520`).

Risk: Lazy BBV is attractive because it has block-level code, but it brings target thunks, property instructions, mutable code, and incomplete-looking version questions (`mir-gen.c:9548-9552`, `mir-gen.c:10001`).

## Runtime Overhead Risks

Risk: Instrumenting every function, block, and edge may make baseline code unrepresentative.

Risk: Timing instrumentation can perturb the behavior it measures.

Risk: Code replacement metadata can grow faster than code if region granularity is too small.

Risk: Keeping old code for fallback can exhaust a simple code arena.

Hypothesis: Start with function counters and code-cache metrics before block/edge counters.

## Documentation Risks

Risk: Design-perspective documents could be mistaken for MIR facts.

Guardrail: Every speculative statement should be marked as `Hypothesis` or `Question`.

Guardrail: Main retrospective docs remain source of truth for MIR.

Risk: Overusing MIR terminology for a new design could hide semantic differences.

Guardrail: Use "MIR-inspired" when discussing future subsets.

## Preservation Risks

Risk: A future design agenda could bias source reading and cause documentation to ignore inconvenient MIR mechanisms.

Risk: Simplifying too early could erase historically important details such as C ABI support, lazy BBV, or code-cache behavior.

Guardrail: Keep archival docs complete and neutral, even when design notes recommend deferring mechanisms.

## Non-Goals

- Do not implement RISC-V32 in this pass.
- Do not rewrite MIR source.
- Do not refactor MIR.
- Do not replace the main documentation.
- Do not define a final fantasy ABI yet.
- Do not claim MIR was designed as a baseline compiled introspective runtime.
- Do not make lazy BBV the default future architecture by assumption.
- Do not make C2MIR compatibility a first-prototype requirement unless a later scope document chooses it explicitly.
- Do not design a full tracing JIT.
- Do not optimize before baseline semantics and observability are testable.

## Practical Guardrails

- Preserve MIR facts separately from future design hypotheses.
- Start with whole-function baseline units.
- Treat interpreter as oracle before treating it as a production tier.
- Collect cheap metrics first.
- Prefer explicit replacement over transparent magic.
- Defer C ABI, varargs, aggregates, and `long double`.
- Defer lazy BBV until whole-function baseline and introspection are understood.
- Keep RISC-V32 and fantasy-bytecode paths separate until requirements are clear.

## Unsupported-Feature Guardrail

Hypothesis: Unsupported features should fail through an explicit taxonomy, not by accidental compiler failure.

Allowed classifications:

- `reject-at-load-time`;
- `lower-to-helper`;
- `interpreter-only`;
- `runtime-trap`;
- `out-of-scope`.

Rule: For MIR-F0, prefer `reject-at-load-time` for validation errors, unresolved internal calls, host C ABI features, indirect calls, lazy BBV, direct-call rewriting, and function redefinition. Prefer `out-of-scope` for floating point, `long double`, varargs, and aggregates unless a later scope document changes this.

## Open Questions

- Question: What minimum prototype would answer whether this design is useful?
- Question: Which workloads justify systematic compilation?
- Question: Which metrics are essential rather than merely interesting?
- Question: Can code replacement be safe without a full deoptimization model?
- Question: How far can the project diverge before it is no longer MIR preservation-adjacent?
