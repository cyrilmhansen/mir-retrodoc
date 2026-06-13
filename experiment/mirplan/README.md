# mirplan

`mirplan` builds deterministic compiler-facing plan artifacts from
`mirspace::ProgramSpace`.

It does not generate code and does not expand MIR-F0 semantics. Its role in F1
is to make the future baseline compiler input explicit, inspectable, and
testable before target-specific lowering exists.
