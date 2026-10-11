# Vim DoubleQuotes Action Classification

**Status:** Vim quote object, `cs` route, and distinct Helix directional adapter classified; directional dispatch and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns quote ranges, selections, and consuming operator effects; Keymap owns the shared profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::DoubleQuotes` as a unit action with no arguments or deprecated aliases. No platform-default map binds it. The shared profile has one quote-key row under `vim_operator == a || vim_operator == i || vim_operator == cs || vim_operator == helix_next || vim_operator == helix_previous`.

In Vim, `i "` and `a "` complete a pending inside/around text-object operator, such as `di"`, `ca"`, or `yi"`. `cs " <new-pair>` uses DoubleQuotes to find the old pair. In Helix mode, `] "` and `[ "` complete around-object Next and Previous routes. Helix current-object `m i`/`m a` routes through `vim::AnyPair`, not `vim::DoubleQuotes`. The action depends on a pending object/operator context; a bare Normal-mode invocation has no object effect. Palette listing and focus behavior are not established here.

## Pinned Vim behavior

`Object::DoubleQuotes` calls the shared `surrounding_markers` scanner with `search_across_lines: false`. It matches literal, unescaped double quote characters on the current buffer row without syntax-tree or language-scope filtering. If the cursor is not within a pair, the scan can choose the next pair to the right on that row. Escaping checks the immediately preceding character for a backslash; the scanner does not count backslash parity. Inside ranges exclude both quote delimiters. Around ranges include them and extend over adjacent horizontal whitespace, preferring whitespace after the closing quote and using whitespace before the opening quote when no trailing whitespace is available. Newlines are not included by that whitespace expansion.

The object ignores the count passed by the surrounding Vim operator. For direct `cs` use, the old delimiter pair is prepared and anchored before the action waits for the replacement delimiter. The direct text-object and surround-change paths share quote classification but have distinct completion and editing effects.

## Pinned Helix behavior

The same quote-key row also completes `helix_next` and `helix_previous` operators. Their `]` and `[` prefixes set `around: true`, and the Helix adapter maps DoubleQuotes to a literal immediate-boundary object. It scans adjacent characters for `"` delimiters and uses its own pair-range algorithm; it does not use Vim's line-limited `surrounding_markers` scanner or syntax-tree query. The boundary scan has no newline or escaped-quote filter, so a directional quote selection may span lines and may treat an escaped quote as a delimiter. These effects are statically visible and still require runtime acceptance.

Helix current `m i`/`m a` is a separate `AnyPair` route whose current pair comes from the language's bracket queries. Tests that select a current quoted pair via `m i m` therefore do not establish DoubleQuotes Next/Previous behavior.

## Pinned fixtures inspected

`test_change_surrounding_character_objects` and `test_delete_surrounding_character_objects` include the double-quote pair in their inside/around operator loops. `test_singleline_surrounding_character_objects` checks Visual inside/around selection, including adjacent whitespace. `test_singleline_surrounding_character_objects_with_escape` checks escaped quotes. `test_change_surrounds_vim` covers `cs " '` and whitespace handling while replacing the old quote pair. `test_select_any_pair_object` covers Helix current quoted-pair selection through `AnyPair`, not DoubleQuotes. No focused Helix DoubleQuotes Next/Previous fixture was found. These pinned-source fixtures were inspected, not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim/Helix profile; Settings owns mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

The local Vim parser has no inside/around quote object scanner, pending object operator, `cs` old-pair route, Helix AnyPair or directional object mode, or matching shared profile entry. A literal quote character in the local editor is not a text-object workflow.

## Acceptance still required

- Verify macOS action registration and `i "`/`a "` in pending delete, change, yank, transform, shell, comment, and surround operators. Verify `cs "` captures the old pair and completes with each supported replacement delimiter.
- Verify same-row pairing, cursor on either delimiter and inside content, next-pair fallback, unmatched and empty pairs, quote escaping with odd/even backslashes, quote-like characters in code/comments/strings, horizontal whitespace on either side, and soft-wrapped rows.
- Verify the action's ignored count behavior for Vim object operators and `cs`; check multi-cursor ranges, Visual expansion, operator register contents, mode/caret transitions, repeat, undo grouping, and recovery when no pair exists.
- Dispatch Helix `] "` and `[ "` separately. Check current pair, next/previous pair order, `around: true`, multi-cursor behavior, escaped quote characters, unmatched pairs, and multiline scan boundaries. Verify `m i m`/`m a m` current-pair behavior separately as AnyPair.
- Verify bare palette invocation and focus behavior if the action is listed. Dispatch shared keymap rows on Linux and Windows separately, and capture paired native macOS Editor and Keymap evidence with mode, operator, quote pair, whitespace, result, theme, and scale recorded.

## Source records inspected

- Action registration and shared key: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`, `zed-refrence/zed/crates/vim/src/object.rs`, and `zed-refrence/zed/assets/keymaps/vim.json`.
- Vim same-row scanner and `cs` old-pair anchors: `zed-refrence/zed/crates/vim/src/object.rs` and `zed-refrence/zed/crates/vim/src/surrounds.rs`.
- Helix directional adapter and boundary algorithm: `zed-refrence/zed/crates/vim/src/helix/object.rs`, `zed-refrence/zed/crates/vim/src/helix/boundary.rs`, `zed-refrence/zed/crates/vim/src/helix/select.rs`, and `zed-refrence/zed/crates/vim/src/normal.rs`.
- Vim quote fixtures and separate Helix AnyPair fixture: `zed-refrence/zed/crates/vim/src/object.rs` and `zed-refrence/zed/crates/vim/src/helix/object.rs`.
- Local Vim route search: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
