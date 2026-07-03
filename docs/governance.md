# Project Governance

This document is the tracked operating guide for keeping `icon-tracer`
reviewable while the tracing and Potrace-parity work continues.

## Document Map

- `AGENTS.md`: short rules for agents and maintainers working in this repo.
- `README.md`: user-facing CLI behavior, presets, and quality harness usage.
- `docs/governance.md`: completion gates, documentation policy, and evidence
  policy.
- `docs/multi-agent-orchestration/`: ignored local planning, review, and
  evidence state for agent sessions.

The ignored orchestration folder is useful working memory, but tracked source
and tracked docs must carry the durable project rules.

## Completion Policy

Every change should leave a reader able to answer three questions:

1. What behavior, workflow, or governance rule changed?
2. Which source or doc files now define that behavior?
3. Which command proves the change did not drift?

Minimum checks before completion:

```sh
scripts/check-governance.py
cargo fmt --check
git diff --check
```

Rust behavior changes:

```sh
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Python script changes:

```sh
/opt/homebrew/bin/python3.12 -m py_compile scripts/check-governance.py scripts/icon-batch.py scripts/potrace-broad-probe.py scripts/potrace-parity.py scripts/potrace_parity_limits.py scripts/vector-quality.py
```

Vector-quality changes:

```sh
./scripts/vector-quality.py --no-build
```

Potrace core parity changes:

```sh
./scripts/potrace-parity.py --no-build --check
```

Broad Potrace-parity changes:

```sh
./scripts/potrace-broad-probe.py --no-build --check
```

Use targeted tests first while developing, then run the relevant full checks
before claiming completion.

GitHub CI runs the minimum governance checks plus Rust format, clippy, tests,
release build, and whitespace checks on pushes to `main` and on pull requests.
Keep local completion evidence aligned with that workflow so CI is a final
guardrail rather than the first place regressions are discovered.

## Reviewability Policy

- Source, script, and test files should stay below 1000 lines.
- Split by existing ownership boundaries: raster parser by format, SVG logic by
  candidate family or serialization concern, tests by fixture family.
- Do not bury durable status only in ignored local docs.
- Do not commit generated files from `target/`.
- Avoid broad refactors when a focused extraction keeps the current change
  readable.

## Potrace-Parity Policy

- The Rust runtime must not shell out to Potrace or vendor Potrace source.
- Local Potrace may be used by scripts as a black-box development oracle.
- Broad probe limits move downward only when a targeted RED check first proves
  the old implementation fails the new limit for the expected fixture.
- Record accepted parity improvements with:
  - fixture and total AE before/after;
  - command, point, or SVG-byte tradeoff when relevant;
  - the targeted test or probe that locked the behavior;
  - the full verification command that passed afterward.
- Rejected experiments should be summarized in local evidence when they explain
  why a tempting shortcut was not adopted.

## Documentation Policy

- User-facing behavior changes belong in `README.md`.
- Project operating rules belong in `AGENTS.md` and this file.
- Agent session state may live in `docs/multi-agent-orchestration/`, but it is
  ignored by Git and should not be the only durable record of project behavior.
- When docs and source disagree, verify source first. Update confirmed docs for
  confirmed behavior; leave uncertain decisions out of confirmed docs until
  there is evidence.

## Governance Check

`scripts/check-governance.py` enforces the lightweight parts of this policy:

- required governance docs exist;
- README points to the governance check;
- Rust/Python source, script, and test files stay below 1000 lines;
- active source/docs surfaces do not retain unresolved open-work markers.

The check is intentionally small. It is a guardrail, not a replacement for the
domain-specific test and parity harnesses above.
