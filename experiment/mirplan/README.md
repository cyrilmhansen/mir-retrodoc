# mirplan

`mirplan` builds deterministic compiler-facing plan artifacts from
`mirspace::ProgramSpace`.

It does not generate code and does not expand MIR-F0 semantics. Its role in F1
is to make the future baseline compiler input explicit, inspectable, and
testable before target-specific lowering exists.

The crate also exposes a target-neutral lowering projection from `CompilePlan`
to `LoweredProgram`. This projection makes value reads, value writes, branch
targets, direct calls, and memory operations explicit without choosing a code
generation target.

Both `CompilePlan` and `LoweredProgram` have deterministic text renderers so
they can be inspected by tests and by `mirtool`.
