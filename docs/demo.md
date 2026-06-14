# Project Demo

This demo is a guided tour of the current MIR-F0/F1 pipeline. It pauses between
steps, prints the input fixture contents, explains what each stage proves, and
ends with the near-term future direction.

## Prerequisites

- Rust toolchain with `cargo`
- Cap'n Proto compiler `capnp`
- Optional host C compiler available as `cc`

If `cc` is not available, run the demo with `--no-cc`.

## Command

```sh
./scripts/demo.sh
```

The default mode pauses between sections for live presentation.

Without a C compiler:

```sh
./scripts/demo.sh --no-cc
```

For a non-interactive terminal recording or CI check:

```sh
./scripts/demo.sh --no-pause
./scripts/demo.sh --no-cc --no-pause
```

## What It Shows

The demo first prints the contents of:

```text
experiment/mircap/tests/fixtures/valid_data_segment_load.mircap.txt
```

This fixture exercises a data segment and byte loads. The expected interpreter
result is:

```text
Result: u32 43
```

The demo then prints the first lines of:

- `mirtool plan`: the MIR-F1 compile-plan artifact.
- `mirtool lower`: the MIR-F1 backend-facing lowered projection.

It also encodes the fixture to Cap'n Proto binary and validates the binary path.
For the size and loading comparison, the script then generates a larger
temporary MIR-F0 module by unrolling a `u32` summation loop until the text
representation is about 20 KiB. The guided demo shows:

- text file size vs binary file size;
- the first text lines beside a hex dump of the binary bytes;
- an in-process loading benchmark for text and binary inputs.

Depending on the module shape, Cap'n Proto schema and framing overhead can still
make the binary larger than text. The useful demo signal is the structured
binary representation on a non-trivial module and the load-time comparison.

When `cc` is available, the demo generates C and runs:

```sh
mirtool diff <fixture>
```

The expected differential result is:

```text
PASS
```

Finally, the demo runs:

```text
experiment/mircap/tests/fixtures/trap_load_oob.mircap.txt
```

The expected trap result is:

```text
Trap: 13 OutOfBoundsLoad
```

The script also prints the trap fixture contents before running it.

## Narrative

The demo shows the current project boundary:

- MIR-F0 is the frozen validated execution subset.
- `mirsem` is the interpreter/oracle.
- `mirc0` is the existing C backend checked against `mirsem`.
- `mirspace` and `mirplan` are F1 analysis and planning layers.
- `mirtool plan` and `mirtool lower` expose the F1 artifacts without changing
  runtime semantics.

The future direction shown at the end is intentionally conservative:

- complete C and differential coverage for the already-started `f32`/`f64`
  arithmetic subset;
- show float results in a deterministic, bit-pattern-friendly way;
- decide RV32FD versus soft-float only after the C/oracle path is stable;
- keep host ABI, varargs, aggregates, lazy versioning, and runtime replacement
  out of the demo-critical path.
