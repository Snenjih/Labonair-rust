# Labonair Editor Testing Plan

This document is the hand-off test plan for the native Editor rework. It is
deliberately separate from the implementation evidence in `tasks/rework/`.
The plan covers the current editor, workspace adapter, command routing, and
the visual states that cannot be proven by compilation alone.

## 1. Automated baseline

Run from the repository root:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
bash scripts/check-crate-deps.sh
python3 scripts/check_rework_queue.py
python3 scripts/check_documentation.py
git diff --check
```

Focused editor runs are useful while iterating:

```text
cargo test -p labonair-editor --all-targets
cargo test -p labonair-workspace --all-targets
cargo test -p labonair-shell --all-targets
cargo clippy -p labonair-editor -p labonair-workspace -p labonair-shell --all-targets -- -D warnings
```

The full workspace test is expected to be evaluated in the local development
environment. At the time this plan was written, two unrelated tests could be
blocked by the restricted runner: the AI SSE socket test and the settings
watcher rename test. Record the exact command and error if either still fails;
do not weaken or skip the test without documenting the environment cause.

## 2. Editor contract and data tests

- Rope edits preserve Unicode scalar positions, UTF-8 byte ranges, CRLF, EOF,
  empty files, and trailing newlines.
- Anchors resolve deterministically at insertions, replacements, and deletions.
- Multi-selection edits apply back-to-front and produce one understandable
  undo/redo transaction.
- Search supports literal, regex, multiline, smart-case, whole-word, replace
  one, replace all, capture groups, wrap-around, and invalid-regex states.
- Project search and File Finder reject stale generations and cancellation;
  paths are revalidated below the active workspace root before opening.
- File lifecycle preserves read-only, binary, too-large, missing, conflict,
  line-ending, BOM, and atomic-save behavior.
- Session records reject unsupported/corrupt versions without overwriting the
  source record and restore path, split, selection, fold, scroll, and recovery
  state where applicable.
- Local language-service results are revision/generation safe and cover
  diagnostics, completion, signature help, hover, navigation, rename, code
  actions, folding, semantic tokens, and formatting fallback.

## 3. Command and keymap tests

Verify that every editor command has one owner descriptor and one executable
route where it is advertised. Exercise the same action through:

1. Command Palette.
2. Native menu entry, where present.
3. Default key binding.
4. A user-overridden key binding.

Minimum commands to check: Open File, Find, Go to Symbol, multi-cursor
commands, line editing, splits, completion, definition/references, rename,
code actions, formatting, diagnostics navigation, Git change navigation, and
Project Diff handoff. Confirm that read-only and unavailable-provider states
disable or explain actions rather than silently doing nothing.

## 4. Manual native UI matrix

Run the native application with `cargo run -p labonair` and record screenshots
or screen recordings for each applicable state. Do not use the legacy
`open -a Labonair` launcher for verification.

### Editor layout

- normal file with syntax highlighting, line numbers, cursor, selection, and
  status information;
- empty/new file;
- narrow window and long path/symbol breadcrumb overflow;
- long file with vertical and horizontal scrolling;
- wrapped lines, rulers, visible whitespace, indent guides, and relative
  numbers;
- manual/syntax fold, sticky context, minimap, outline, and Git gutter;
- one editor group, nested horizontal/vertical splits, resize, focus, close,
  and restored split state;
- visible keyboard focus, hover, disabled, selected, and pressed states.

### Search and navigation

- current-file search with literal/regex/whole-word/smart-case/multiline;
- replace-one and replace-all with one undo step;
- project search loading, results, empty, error, truncated, and result-open;
- Open File loading, fuzzy results, no match, empty root, truncated list,
  keyboard navigation, click-open, Escape, and stale-result safety;
- outline/breadcrumb symbol navigation and Go to Symbol.

### Language and file lifecycle

- completion loading/results/documentation/selection and Escape;
- signature-help loading/ready/empty/unavailable/error near the cursor;
- hover, diagnostics, code actions, rename, formatting, and language-service
  crash/unavailable fallback;
- clean external reload, dirty conflict, keep mine, reload, missing, binary,
  too-large, read-only, retry, and recovery restore/discard;
- autosave and format-on-save enabled, disabled, delayed, cancelled, failed,
  read-only, and conflict cases.

### Integration surfaces

- Git gutter navigation and hunk context menu handoff to canonical Project
  Diff/SCM;
- Editor commands through the global menu and command palette;
- normal statusbar notification dropdown for errors/actions; confirm that no
  toast or duplicate shell status surface appears;
- session restart with restored editor path, cursor/selection, folds, scroll,
  splits, and explicit unsaved recovery decision.

## 5. Performance and safety checks

- Open a 100,000-line text file and verify typing, cursor movement, scrolling,
  folding, and search remain responsive.
- Open a binary file and a file above the configured size limit; verify that no
  lossy placeholder text is inserted into an editable buffer.
- Search a repository containing ignored directories and symlinks; verify the
  result cap, root boundary, cancellation, and no traversal outside the root.
- Change or delete a file while it is dirty; verify that no save silently
  overwrites the external version.
- Exercise malformed session and language-service responses; verify a typed
  error state and continued editor usability.
- Confirm logs, notifications, persistence, and test fixtures contain no
  secrets or full sensitive file contents.

## 6. Explicitly deferred from this plan

AI editor features, remote language services, collaboration, extension/
marketplace hosting, debugger/tasks, and remote theme downloads are not part
of this rework pass. They require separate product decisions and owners.
Inlay Hints and Code Lens should be added only when a real provider contract
and user workflow are introduced; they must not be represented by fake data.

## 7. Sign-off record

Record the date, platform, commit/worktree state, commands run, visual cases
captured, failures, and whether each failure is a product defect or an
environment limitation. Link screenshots or recordings from the acceptance
audit rather than marking a visual case complete from a passing build alone.
