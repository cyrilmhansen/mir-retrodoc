# Floating-Point Implementation Roadmap

This roadmap expands the experimental MIR-F0/F1 rewrite toward `f32` and `f64`
support without changing the current integer-first backend contract too early.

## Phase 1: MIR-F0 Float Constants

Target: `mircap`

Status: implemented.

- Add `f32` and `f64` type identifiers to the text and Cap'n Proto formats.
- Add immediate operands for IEEE-754 single and double precision values.
- Validate `const_f32` and `const_f64` instructions.
- Round-trip float constants through text -> `ModuleImage` -> Cap'n Proto ->
  `ModuleImage`.

This phase established the serialized contract and validation surface before
float arithmetic was made executable.

## Phase 2: Semantic Oracle

Target: `mirsem`

Status: implemented for constants and arithmetic.

- Extend interpreter values with `F32` and `F64` bit-pattern values.
- Execute `const_f32`, `const_f64`, and basic arithmetic.
- Add fixtures that return float values and compare exact bit patterns where
  appropriate.

## Phase 3: C Differential Path

Target: `mirc0`

Status: implemented for constants and arithmetic.

- Map `f32` to C `float`.
- Map `f64` to C `double`.
- Emit float constants and arithmetic.
- Differentially verify C execution against `mirsem`.
- Print float results with both decimal text and exact IEEE-754 bits.
- Include float fixtures in `mirtool diff` and the C column of `diff-all`.

## Phase 4: Comparisons And Conversions

Targets: `mircap`, `mirsem`, `mirc0`

- Add float comparisons only after NaN behavior is specified.
- Add integer/float conversions only after invalid and out-of-range behavior is
  specified.
- Keep conversion traps explicit if the chosen policy differs from native C.

## Phase 5: Planning And Optimization

Target: `mirplan`

- Represent float reads, writes, and constants in lowered plans.
- Add local constant propagation for float constants.
- Add constant folding only for operations whose IEEE behavior is explicitly
  specified.

## Phase 6: RV32 Backend

Target: `mirrv32`

- Start with a declared backend mode: soft-float helper calls or RV32F/RV32D.
- Prefer soft-float until the calling convention and register allocator model
  are stable.
- Add RV32F/RV32D register allocation after integer spilling and call handling
  are no longer moving targets.
