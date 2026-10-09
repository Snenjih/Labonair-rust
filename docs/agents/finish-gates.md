# Agent Finish Gates

**Status:** Agent workflow guide
**Owner:** Repository maintainers

A change is complete only when the gates required by its scope are run and
their result is recorded. A gate may be marked unavailable only with the
reason, environment, and follow-up owner.

## Always required

For every repository change:

```text
git diff --check
python3 scripts/check_documentation.py
```

For changes to `tasks/rework/`, also run:

```text
python3 scripts/check_rework_queue.py
```

## Architecture and dependency changes

```text
scripts/check-crate-deps.sh
python3 scripts/verify.py --scope architecture
python3 scripts/verify.py --scope agents
```

Record every new or removed internal edge and its reason. Transitional edges
must name a removal task and condition.

For structured product knowledge changes, also run the matching scope:

```text
python3 scripts/verify.py --scope capabilities
python3 scripts/verify.py --scope surfaces
python3 scripts/verify.py --scope workflows
python3 scripts/verify.py --scope settings
python3 scripts/verify.py --scope performance
python3 scripts/verify.py --scope visual
python3 scripts/verify.py --scope evidence
python3 scripts/verify.py --scope automation
python3 scripts/verify.py --scope release
python3 scripts/verify.py --scope knowledge
```

Capability descriptors, command/surface catalogs, and scorecard records must
keep generated views fresh and must record missing visual, security, or
performance evidence explicitly.

Changes to the reform plan or its traceability source must also pass
`python3 scripts/verify.py --scope reform`; the generated reform-coverage view
must remain fresh and every definition-of-done record must name a bounded next
task.

## Rust changes

Run the focused package tests first, then the full required gates:

```text
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Do not hide an unrelated baseline failure. Report the exact package, file,
diagnostic, and whether the failure was introduced by the change.

## UI and layout changes

In addition to Rust and documentation gates:

1. Run the native Rust application with `cargo run -p labonair` or an exact
   native binary path.
2. Verify the applicable normal, narrow, focused, empty, loading, error,
   long-list, and overlay states.
3. Record the exact binary, PID/window, date, viewport condition, screenshot
   path or exact reproduction note.
4. Keep states pending when the native window or capture permission is not
   available.

## Settings and persistence changes

Verify defaults, scope, load/save/reload, malformed input, migration, and
unknown-key behavior. Record the data owner and whether restart or live reload
is required.

## Remote, MCP, and security changes

Verify both allowed and denied paths. Include grants, host/session boundaries,
rooted paths, timeouts, cancellation, error redaction, and secret handling.
Never use real credentials or host data in fixtures.

## Performance changes

Record the measurement method, input size, environment, baseline, budget, and
variance. A qualitative claim is not a performance baseline.

## Handoff rule

Use [`handoff-template.md`](handoff-template.md). The handoff must list all
failed or unavailable gates, not only successful checks.
