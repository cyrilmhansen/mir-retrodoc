# Upstream MIR Attribution

This repository studies and experiments with ideas inspired by Vladimir
Makarov's MIR lightweight JIT compiler.

## Upstream Project

- Upstream repository: <https://github.com/vnmakarov/mir>
- Author: Vladimir Makarov
- Upstream license: MIT
- Upstream copyright notice: Copyright (c) 2018-2024 Vladimir Makarov

The upstream MIR repository is live and public. `mir-retrodoc` does not vendor
or mirror the upstream source tree. Readers should use the upstream repository
as the canonical source for MIR code and licensing.

## Scope Of This Repository

`mir-retrodoc` contains:

- retrospective notes about upstream MIR behavior;
- design-perspective notes for MIR-inspired runtime experiments;
- Rust experimental crates for the MIR-F0 and MIR-F1 tracks.

MIR-F0 and MIR-F1 are not full MIR and are not upstream MIR. They are explicit
MIR-inspired experimental subsets.

## Local Preservation Archive

Some historical notes mention paths such as `mir-preservation/git/mir-restored`.
Those paths refer to a local, non-versioned preservation checkout used during
early documentation work. That archive is not part of this public repository.

When adding new source-grounded claims, prefer citations in this form:

- upstream repository URL;
- upstream commit SHA when known;
- file path and symbol name;
- line number only as secondary context, since upstream line numbers can drift.

## Archive Policy

The public git repository intentionally avoids committing large preservation
bundles, generated archives, or mirrored upstream source trees. If a fixed
archive snapshot becomes necessary later, publish it as a GitHub release asset
with checksums rather than adding it to normal git history.
