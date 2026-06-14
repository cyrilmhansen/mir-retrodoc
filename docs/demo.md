# Project Demo

This demo is a short tour of the current MIR-F0/F1 pipeline. It uses one valid
fixture and one trap fixture to show validation, interpretation, planning,
lowering, binary encoding, C output, and differential checking.

## Prerequisites

- Rust toolchain with `cargo`
- Cap'n Proto compiler `capnp`
- Optional host C compiler available as `cc`

If `cc` is not available, run the demo with `--no-cc`.

## Command

```sh
./scripts/demo.sh
```

Without a C compiler:

```sh
./scripts/demo.sh --no-cc
```

## What It Shows

The demo validates and runs:

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

## Narrative

The demo shows the current project boundary:

- MIR-F0 is the frozen validated execution subset.
- `mirsem` is the interpreter/oracle.
- `mirc0` is the existing C backend checked against `mirsem`.
- `mirspace` and `mirplan` are F1 analysis and planning layers.
- `mirtool plan` and `mirtool lower` expose the F1 artifacts without changing
  runtime semantics.

