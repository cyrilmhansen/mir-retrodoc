# Public Release Notes

This repository is a public preservation and experimental rewrite workspace for
MIR.

## Current Public State

- `MIR-F0 v0` is implemented and documented.
- `mircap`, `mirsem`, `mirc0`, `mirtool`, `mirspace`, and `mirplan` all have
  passing test suites in the current workspace state.
- `mirspace` provides the indexed analysis layer used by `mirplan`.
- `mirplan` provides deterministic compile-plan artifacts and text rendering.
- `mirtool plan` exposes the plan artifact through the CLI.
- GitHub Actions CI runs `./scripts/test-all.sh` and
  `./scripts/fmt-all.sh --check`.

## Public Boundaries

- Upstream MIR is referenced and credited separately in `ATTRIBUTION.md` and
  `docs/upstream-mir.md`.
- This repository does not vendor the upstream MIR source tree.
- Unsupported upstream MIR behavior is rejected or documented explicitly.

## F0 / F1 Boundary

- F0 is the frozen validated subset.
- F1 begins with `mirspace` analysis and `mirplan` compile-plan artifacts.
- F1 work is intentionally conservative and does not expand the language
  surface first.

## Validation Commands

```sh
./scripts/test-all.sh
./scripts/fmt-all.sh --check
```

