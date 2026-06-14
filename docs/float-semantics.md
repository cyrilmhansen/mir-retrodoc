# Floating-Point Semantics

This document defines the current MIR-F0 floating-point contract. It is
intentionally narrow.

## Supported Types

- `f32`: IEEE-754 binary32.
- `f64`: IEEE-754 binary64.

The legacy `float` textual type remains an unsupported upstream-MIR marker and
is still rejected by MIR-F0 validation. New fixtures must use `f32` or `f64`.

## Text Syntax

Float constants use explicit operand tags so they do not conflict with function
references:

```text
type 1 f32
type 2 f64
insn 1 const_f32 r:0 f32:1.5
insn 2 const_f64 r:1 f64:-0.25
```

The text loader currently accepts Rust's standard decimal float parser for
finite values and standard special spellings supported by that parser.

## Current Executable Surface

Supported in `mircap` validation:

- `const_f32`
- `const_f64`
- return, call, and copy type checking over `f32`/`f64` values

Reserved but not executable yet:

- float arithmetic
- float comparisons
- integer/float conversions
- float memory load/store
- float C emission
- float RV32 code generation

## Edge-Case Policy

Finite constants, `-0.0`, infinities, and NaNs are represented as IEEE bits in
the in-memory `ModuleImage`.

The following policies are not yet part of MIR-F0 execution:

- NaN comparison behavior
- signaling NaN behavior
- float exception flags
- rounding mode control
- invalid float-to-int conversions
- out-of-range float-to-int conversions

Those choices must be specified before enabling comparisons or conversions.
