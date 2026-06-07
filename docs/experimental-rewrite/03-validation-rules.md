# Validation Rules

The initial validator is structural and conservative.

## Implemented

- supported format version;
- unique IDs per namespace;
- valid references;
- block ownership;
- block terminator presence;
- no instructions after terminators;
- branch targets in the same function;
- direct-call arity/result checks;
- return value count checks;
- conservative operand/result shape checks;
- value type table checks;
- minimal memory opcode type checks;
- data segment structure checks;
- explicit rejection of unsupported types/opcodes.

## Not Yet Implemented

- full value type propagation;
- dominance checks;
- CFG reachability policy;
- execution-time memory access traps;
- source-span validation;
- Cap'n Proto binary decoding.

## Failure Policy

Unsupported MIR-F0 features should fail explicitly at load/validation time.
Silent fallback is not part of the module-image contract.
