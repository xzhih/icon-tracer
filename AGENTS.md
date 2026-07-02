# Project Governance

This repository is a Rust CLI/library for icon-oriented bitmap-to-SVG tracing.
Keep project governance lightweight, source-backed, and easy to verify.

## Sources Of Truth

- `README.md` describes user-facing behavior, CLI flags, presets, and quality
  harnesses.
- `docs/governance.md` defines completion gates, documentation policy, and
  reviewability rules for project work.
- `docs/multi-agent-orchestration/` is ignored local workflow state. It can hold
  useful planning and evidence during agent work, but tracked docs and source
  files must not depend on it as the only record of important behavior.
- Generated reports and rendered probes under `target/` are diagnostic output.
  Do not commit them.

## Completion Gates

Before claiming repository work is complete, run the narrow checks that match
the change and always run:

```sh
scripts/check-governance.py
cargo fmt --check
git diff --check
```

For Rust behavior changes, also run:

```sh
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

For vector quality or Potrace-parity behavior changes, also run the relevant
harnesses documented in `README.md` and `docs/governance.md`.

## Reviewability Rules

- Keep source, script, and test files below 1000 lines. Split a file before
  adding more logic if it would cross that limit.
- Keep behavior changes evidence-gated. Lower parity limits only after a RED
  check fails for the expected reason and the implementation makes it pass.
- Keep the runtime independent from Potrace. The local Potrace CLI may be used
  only by development scripts and evidence generation.
- Prefer small, reversible changes that match the existing module boundaries.
- Remove transient debug output and generated artifacts before finishing.

## Documentation Rules

- Update `README.md` when user-visible CLI behavior, presets, or quality
  harness usage changes.
- Update `docs/governance.md` when completion policy, evidence policy, or
  project operating rules change.
- Record broad, uncertain, or local-only agent workflow context under
  `docs/multi-agent-orchestration/` when helpful, but keep durable tracked
  governance in this file and `docs/governance.md`.
