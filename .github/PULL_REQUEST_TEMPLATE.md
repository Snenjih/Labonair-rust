<!--
PR title should follow Conventional Commits — it becomes the squash commit message.
Examples: feat(terminal): add split panes / fix(explorer): close button alignment
-->

## What
<!-- One or two sentences describing the change. -->

## Why
<!-- The problem you're solving. Link to the issue if there is one (e.g. "Closes #42"). -->

## How
<!-- Brief notes on the approach, only if non-obvious. -->

## Testing
<!-- How did you verify this works? "Ran tsc clean" is not enough on its own —
     describe the actual flows you exercised. -->

- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo check --workspace --all-targets` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace --no-fail-fast` clean
- [ ] Manual smoke-test of the affected feature in the native Rust app
- [ ] `scripts/check-crate-deps.sh` clean when dependencies changed
- [ ] `python3 scripts/check_documentation.py` clean when documentation or control metadata changed
- [ ] `python3 scripts/check_rework_queue.py` clean when the active queue changed

## Screenshots / GIFs
<!-- Required for any UI change. Use the native Rust bundle and record the
     inspected state; never use the legacy installed app. -->

## Notes for reviewer
<!-- Anything risky, anything you want a second opinion on, follow-ups for later. -->
