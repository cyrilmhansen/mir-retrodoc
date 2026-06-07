# Differential Testing

`mirc0` features an integration test suite that performs differential testing against `mirsem`.

## Testing Process

1. Parse MIR-F0 fixture to `ModuleImage`.
2. Run with `mirsem`, capturing return value or trap code.
3. Compile generated C with `cc` on the host, overriding memory sizes if needed.
4. Execute generated binary.
5. Capture exit status, stdout, and stderr.
6. Compare output values and trap codes/identities.
