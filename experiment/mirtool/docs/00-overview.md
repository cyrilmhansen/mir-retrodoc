# mirtool Overview

`mirtool` is a developer utility designed to exercise the MIR-F0 experimental rewrite pipeline. It acts as a single CLI entry point wrapper for:
- Static analysis and validation of ModuleImages (`mircap`).
- Serialization format conversion (Text `.mircap.txt` to/from Cap'n Proto `.mircap` binary).
- Strict reference interpretation (`mirsem`).
- Strict transpilation to portable C11 (`mirc0`) and differential host verification.

## Scope & Target Audience
- **Target Audience**: Compiler developers, pipeline testers, and static analysis contributors.
- **Production Exclusions**: This tool is explicitly *not* a production compiler backend or a high-performance JIT environment. It is used to guarantee compilation and execution correctness via differential oracle validation.
- **Dependencies**: Keeps dependencies extremely minimal by parsing arguments manually via standard library APIs, avoiding third-party parsing libraries.
