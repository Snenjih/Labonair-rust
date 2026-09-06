# Vergleichsbericht — Subagent 3

Scope: Command palette, context menus, icon system (emoji vs. real icons), theme system, dynamic sidebars.

Method: read of `reference-src/src/modules/command-palette/**`, `shortcuts/**`, `theme/**`, `settings/useThemeStore.ts`, `src-tauri/src/modules/themes/**`, `statusbar/lib/{useSidebar,sidebarSlotLogic}.ts`, every `ContextMenu`/`onContextMenu` site under `reference-src/src`, vs. the port in `crates/ui/src/{command_palette,theme,syntax_theme,menu,app_shell,workspace,pane,explorer,sftp,settings}.rs` and `crates/theme/**`. Full emoji sweep of `crates/**`.

---

## 0. Executive summary

| Area | Verdict |
|---|---|
| Command palette | Data layer is a faithful but **heavily reduced** port (~34 static commands vs. ~90+ dynamic actions across 14 hook modules). View layer is **visually wrong and structurally incomplete**: 1 hard-coded sub-page vs. 11, no icons, no keybind-hint column, no recents, no `rightLabel` (ON/OFF/active), no footer, cramped metrics, substring-only search, no palette preferences. |
| Icons | Reference uses **SVG icon sets everywhere** — `@hugeicons/react` for UI/commands/tabs, the full **Catppuccin iconify set** for file/folder icons. Port has **no icon crate at all** (`gpui-component` is not even a dependency) and substitutes ~30 Unicode geometric glyphs + ~12 real emoji (📁 📄 🦀 📜 ⚙ 📝 🖼️ ✂ ✨ ⛓ 🛡 🔍 🗑 📎 🔗). |
| Theme system | Built-in light/dark + System + **one** imported custom theme file + export + editor syntax themes are done. **Missing:** multi-installed-theme list is half-wired (settings only, not palette), **named theme-variant selection** (Catppuccin frappe/macchiato/mocha all share `mode:"dark"` — port always picks the first), **`themeVariantOverrides` persistence**, the entire **community/marketplace** flow (`theme_fetch_index`/`theme_download`/`install`/`uninstall`/`MOCK_COMMUNITY_THEMES`), **`theme_create` ("New Theme")**, and **live hover-preview** of a theme from the palette. |
| Dynamic sidebars | Reference sidebars are **fully dual-dock**: two independent slots (left/right), any panel dockable to any side, per-panel `barItemPlacements[side]`, move-panel, drag-resize with debounced width persistence, restore-from-prefs, `tabs`-in-sidebar and `hosts` panels. Port has **one fixed left-only sidebar**, 5 panels, no right dock, no move, no per-item placement, no active-panel/width persistence. Not dynamic at all. |

---

## 1. Command palette

### 1a. Reference behaviour

**Registry** — `reference-src/src/modules/command-palette/useCommandRegistry.ts:19-113` composes 14 hook modules into a `root` page plus **11 named sub-pages**: `hosts-ssh`, `hosts-sftp`, `themes`, `mode`, `editor-theme`, `zoom`, `tabs`, `snippets`, `ai-sessions`, `outline`, `git-branches` (`:72-85`). Sub-pages are reached via `action.subPageId` and rendered as a **breadcrumb trail** with clickable crumbs (`CommandPalette.tsx:284-309`), `Backspace`/`Esc` pop one level (`:127-147`).

**Action shape** — `types.ts:6-18`: `id`, `title`, `subtitle?`, `icon?` (ReactNode), `shortcut?` (string[] key tokens), `section`, `contexts?`, `perform?`, `subPageId?`, `rightLabel?`, `onPreview?`.

**Full reference command inventory** (dynamic — counts vary with app state):

| Hook (file) | Root actions | Sub-page |
|---|---|---|
| `useSystemCommands.ts` | Open Settings, Keyboard Shortcuts, Toggle AI Panel, Ask AI About Selection, Manage AI Keys & Models | — |
| `useLayoutCommands.ts` | New Terminal Tab, New Editor Tab, Open Host Manager, Open SFTP… (→hosts-sftp), Duplicate Tab, Close Other Tabs, (+ if workspace) Split Pane Right, Split Pane Down, Close Active Pane | — |
| `useHostCommands.ts` | Quick Connect (≤3 recent hosts × SSH+SFTP), Connect SSH… (→hosts-ssh), Open SFTP… (→hosts-sftp), Add New Host… | hosts-ssh (1 per host), hosts-sftp (1 per host) |
| `useSettingsCommands.ts` | Change Editor Theme… (→editor-theme), Change App Theme… (→themes), Change Color Mode… (→mode), + 17 `Toggle:` actions (Word Wrap, Auto-Save, Cursor Blink, WebGL, Pane Header, Pane Footer, Vim Mode, Show Hidden Files, AI Autocomplete, Launch at Login, Show Header Bar, Show Status Bar, Zen Mode) each with `rightLabel` ON/OFF | themes (default + installed, `onPreview`+`onLeave` revert), mode (Dark/Light/System), editor-theme (`EDITOR_THEMES`) |
| `useZoomCommands.ts` | Adjust Font Size… (→zoom, only if terminal/editor/sftp active) | zoom (Increase / Decrease / Reset, with `<n>px` subtitle) |
| `useTabCommands.ts` | Switch Tab… (→tabs), Close Current Tab | tabs (1 per open tab + kind icon + "active" label, then Close Current Tab) |
| `useSnippetCommands.ts` | Snippets… (→snippets), Manage Snippets | snippets (1 per snippet, grouped by group name, `rightLabel` = exec mode) |
| `useAiSessionCommands.ts` | New AI Session, Clear Current Chat, Switch AI Session… (→ai-sessions, if any) | ai-sessions (1 per session, "active" label) |
| `useTerminalCommands.ts` | Clear Terminal (ctx terminal/ssh-terminal), Disconnect SSH, Reconnect SSH (ctx ssh-terminal) | — |
| `useSftpCommands.ts` | Copy Remote Path, Copy Local Path (ctx sftp, `subtitle` = path) | — |
| `useEditorCommands.ts` | Go to Symbol… (→outline), Toggle: Word Wrap / Line Numbers / Bracket Matching / Format on Save / Indentation Guides / Code Outline, Format Document (⌘⇧F) — all ctx editor | outline (1 per symbol, `Ln n` subtitle, empty state "No symbols found") |
| `useSourceControlCommands.ts` | Open Git Graph, Focus Source Control, + if repo: Git Switch Branch… (→git-branches), Push, Pull, Fetch, Force Push (with-lease), Stage All, Unstage All, Stash Changes, + Pop Latest Stash (if stash) | git-branches (Local/Remote sections, "current" label, empty state) |
| `useExplorerCommands.ts` | Refresh File Tree, Hard Refresh File Tree, Toggle: Show Hidden Files (Explorer), New File in Explorer, New Folder in Explorer, Reconnect Explorer Sessions, Copy Explorer Root Path | — |

**Context filtering** — `useCommandRegistry.ts:45-48`: no-context actions always show; context-scoped only when `activeContext ∈ {terminal, editor, sftp, home, ssh-terminal}` matches.

**View styling** (`CommandPalette.tsx`):
- Dialog: `max-w-[640px]`, `rounded-2xl`, `border-border/60`, `shadow-modal`, overlay `bg-black/40` + configurable `backdrop-blur`, configurable open animation (`none|fast|130ms|slow`), configurable vertical position (`high 8% / default 15% / center 50%`), configurable card `opacity` via `color-mix` (`:225-253`).
- Header: `h-14`, search icon or breadcrumb, input `text-[15px]` (`:283-324`).
- List: `max-h-96`, `p-2`, animated slide between pages via `motion` (`:327-370`).
- Section group heading: `px-3 py-2 text-[10px] font-medium uppercase tracking-widest text-muted-foreground/70` (`:344, :361`).
- **"Recently Used"** group shown first when `showRecent` pref on and no query (`:341-355`, `useCommandStore` `recentIds`/`pushRecent`).
- Item (`PaletteItem`, `:415-472`): `min-h-10`, `rounded-lg`, `gap-3`, `px-3 py-2`, `mx-0.5 my-0.5`, **`border-l-2 border-transparent`** → selected `border-primary bg-accent/60 pl-[10px]`; `size-7` rounded icon chip; title `text-[13px] font-medium` + optional `subtitle` `text-[11px] text-muted-foreground`; right side: **`rightLabel`** (`text-[10px] font-bold uppercase`, green for ON/active) + **`shortcut` as `<Kbd>` chips** + chevron for sub-pages.
- Empty state: `py-10 text-center text-[13px]` "No results found." (`:337-339`).
- Footer (`:373-401`): clickable **search-mode toggle** (`contains`/`startsWith`/`fuzzy`, cycles, persisted `commandPaletteSearchMode`), `·` separator, **result count** ("N results"), right side `↑↓ navigate` / `↵ select` / `⌫ back` (when nested) / `Esc close` as `<Kbd>`.
- Search: `contains` | `startsWith` | `fuzzy` (subsequence) over `title + subtitle + section` (`:194-211`, `:261-279`).
- `onPreview` fires on highlight change (theme live-preview); `onLeave` fires when leaving a page (theme revert).
- Palette preferences (all in `usePreferencesStore`): `commandPaletteBlur`, `commandPaletteOpacity`, `commandPalettePosition`, `commandPaletteAnimation`, `commandPaletteShowRecent`, `commandPaletteSearchMode`, `commandPaletteCloseOnOverlayClick`.

**Shortcut model** — `reference-src/src/modules/shortcuts/shortcuts.ts`: 28 entries feeding `useShortcutHint(id)` → key-token array shown in the palette's hint column. Groups: General, Tabs, Search, AI, View, Bookmarks.

### 1b. Port current state

`crates/ui/src/command_palette.rs`:
- Shortcut table `SHORTCUTS` (`:97-129`) — **28 entries, 1:1 with the reference**, plus `find_conflict`/`resolve_conflict`/`effective_binding` override machinery (well done, fully unit-tested). One divergence: `ShortcutsOpen` is bound to **`cmd-k` / `⌘K`** (`:100`) while the reference is **`⌘?` / `cmd-?`** (`shortcuts.ts:65-68`).
- Static `COMMANDS` registry (`:378-413`) — **34 commands, 12 sections**. Purely static: no per-host, per-snippet, per-session, per-branch, per-symbol, per-theme rows; no `Quick Connect`; no `rightLabel`; no `subtitle`; no `icon`; no `onPreview`.
- `Page` enum (`:487-491`) has exactly **two** variants: `Root`, `SwitchTab`. The other 10 reference sub-pages **do not exist** (no themes / mode / editor-theme / zoom / snippets / ai-sessions / outline / git-branches / hosts-ssh / hosts-sftp).
- `search()` (`:443-453`) is **substring-only** over `title + section` — no `startsWith`, no `fuzzy`, no search-mode toggle, `subtitle` not searched.
- View `render` (`:686-811`): overlay `crate::theme::modal_scrim()`, card **`w(520px)`**, `max_h(440px)`, `rounded_md`, header **`h(36px)`** `text_size(12px)`, list `max_h(360px)` `py(4px)`, section heading `px(12) pt(6) pb(2) text_size(9px)` (not uppercase-tracked style, just `.to_uppercase()`), row **`h(26px)`** `text_size(12px)` `rounded_sm`, selected = `bg(selected_fill)`, hover = same fill. Right side renders **only** `hint` as plain `text_size(10px)` text (space-joined key tokens, e.g. `⌘ P`) — **no `<Kbd>`-style chips**.
- **No** icons, **no** recents, **no** `rightLabel`, **no** subtitle, **no** footer (no result count, no nav hints, no search-mode toggle), **no** breadcrumb (SwitchTab page just swaps placeholder text), **no** animations, **no** palette preferences, **no** live preview, **no** empty-state copy beyond "No matching commands" (`:718-726`).
- Keyboard: `esc` (pop SwitchTab → root, else close), `enter`, `up`/`down` (wrap), `backspace`, char input (`:612-664`). No `⌫`-to-go-back affordance shown.

### 1c. Concrete fixes (command palette)

1. **Widen + restyle to match the reference metrics.** Card `w 640px` / `rounded_2xl` / `border_color(border @ 0.6)` / modal shadow; header `h 56px`, input `text_size 15px`; list `max_h 384px` + `p 8px`; row `min_h 40px` / `rounded_lg` / `gap 12px` / `px 12 py 8` / `mx 2 my 2`, left accent bar `border_l 2px` transparent → `primary` when selected with `bg(accent @ 0.6)` and `pl 10px`; section heading `px 12 py 8 text_size 10px` uppercase + wide letter-spacing at `muted @ 0.7`; empty state `py 40px text-center text_size 13px` "No results found."
2. **Add the footer**: left = clickable search-mode pill (`contains`/`startsWith`/`fuzzy`), middle = "N results", right = `↑↓ navigate` · `↵ select` · `⌫ back` (nested only) · `Esc close`, rendered as bordered key chips.
3. **Implement all 11 sub-pages** as a generic `Page { id, placeholder, actions: Vec<PaletteRow>, on_leave }` stack with a **breadcrumb trail** (clickable crumbs, `Backspace`/`Esc` pop). Replace the `Page` enum with a `Vec<PageId>` stack.
4. **Make the registry dynamic.** Port the 14 hook modules as functions that take app state (`&Workspace`, hosts store, snippets store, chat sessions, source-control store, editor outline, theme list, preferences) and return `Vec<CommandAction>`; re-run on every render like `useCommandRegistry`. Add the missing domains: Quick Connect, per-host SSH/SFTP pages, per-snippet, per-AI-session, per-branch, Go-to-Symbol/outline, the ~17 `Toggle:` settings actions with `rightLabel`, Manage AI Keys & Models, Git Push/Pull/Fetch/Force-Push/Stage-All/Unstage-All/Stash/Pop, Explorer refresh/new-file/new-folder/hidden/reconnect/copy-root.
5. **Add `icon`, `subtitle`, `rightLabel`, `on_preview` to the command/row model.** Render the `size-7` icon chip (needs the icon system — see §2), the subtitle line, and the green `rightLabel` for ON/active.
6. **Add fuzzy + startsWith search** and a persisted `command_palette_search_mode` preference; search over `title + subtitle + section`.
7. **Add "Recently Used"** group (persist `recent_ids`, push on execute, show first when no query on root) gated on a `command_palette_show_recent` preference.
8. **Add palette preferences**: blur, opacity, position (high/default/center), animation, close-on-overlay-click, show-recent, search-mode — wire into Settings.
9. Fix `ShortcutsOpen` binding to `cmd-?` (or confirm the deliberate divergence in `bugs_and_fixes.md`).
10. Wire `on_preview`/`on_leave` for the themes page so hovering a theme live-applies and leaving reverts (reference `themesPage.onLeave = revertToSavedTheme`).

---

## 2. Icon system (emoji vs. real icons)

### 2a. Reference

- **UI / command / tab / status icons**: `@hugeicons/react` (`HugeiconsIcon` + `@hugeicons/core-free-icons`). Every command action builds `createElement(HugeiconsIcon, { icon: XxxIcon, strokeWidth: 2, className: "size-4" })`. Tab icons: `tabs/lib/tabUtils.tsx:16-27` (`ComputerTerminal02Icon`, `PencilEdit02Icon`, `Folder01Icon`, `Globe02Icon`, `GitBranchIcon`, `GitCompareIcon`, `Home03Icon`, `CloudServerIcon`).
- **File / folder icons**: the **full Catppuccin icon set** via `@iconify-json/catppuccin` — `reference-src/src/modules/explorer/lib/iconResolver.ts` resolves a file name / extension / language-id / folder name to an SVG body and emits a `data:` URL rendered as `<img class="size-4">` (`FileTreeNode.tsx:80,105,202`). Mapping tables: `fileIcons.ts`, `folderIcons.ts`, `constants.ts` (`EXT_TO_LANGUAGE_ID`).
- The reference **never uses emoji** anywhere.

### 2b. Port

- **No icon crate.** `gpui-component` (named in the architecture doc) is **not in `Cargo.toml`** (`crates/ui/Cargo.toml` has no `gpui-component`, no `lucide`, no SVG icon dep). No `IconName`, no `Icon::`, no SVG asset pipeline in `crates/ui/src`.
- Icons are faked with Unicode. Full inventory of glyphs used as icons:

**Real emoji (pictographs) — must be replaced:**

| file:line | glyph | used for | proper icon (hugeicons parity) |
|---|---|---|---|
| `crates/ui/src/explorer.rs:939` | 🦀 `\u{1F980}` | `.rs` files | Catppuccin `rust` / lang icon |
| `crates/ui/src/explorer.rs:940` | 📜 `\u{1F4DC}` | js/ts/jsx/tsx | Catppuccin `javascript`/`typescript`/`typescript-react` |
| `crates/ui/src/explorer.rs:941` | ⚙ `\u{2699}` | json/toml/yaml/lock | Catppuccin `json`/`toml`/`yaml` |
| `crates/ui/src/explorer.rs:942` | 📝 `\u{1F4DD}` | md/txt | Catppuccin `markdown`/`text` |
| `crates/ui/src/explorer.rs:943` | 🖼️ `\u{1F5BC}` | png/jpg/gif/svg/webp | Catppuccin `image` |
| `crates/ui/src/explorer.rs:944` | 📄 `\u{1F4C4}` | default file | Catppuccin `file` |
| `crates/ui/src/explorer.rs:1022,1332` | 📁 `\u{1F4C1}` (+ `＋`) | folder / new-folder button | Catppuccin `folder` / `folder-open` |
| `crates/ui/src/explorer.rs:1028` | ↻ `\u{21BB}` | refresh button | hugeicons `Refresh01Icon` |
| `crates/ui/src/app_shell.rs:104` | 📁 `\u{1F4C1}` | sidebar rail: Explorer | hugeicons `Folder01Icon` |
| `crates/ui/src/app_shell.rs:105` | ✂ `\u{2702}` | sidebar rail: Snippets | hugeicons `CommandIcon` |
| `crates/ui/src/app_shell.rs:106` | ⌥ `\u{2325}` | sidebar rail: Source Control | hugeicons `GitBranchIcon` |
| `crates/ui/src/app_shell.rs:107` | ⛓ `\u{26D3}` | sidebar rail: Git Graph | hugeicons `GitBranchIcon`/`GitCompareIcon` |
| `crates/ui/src/app_shell.rs:108` | ✨ `\u{2728}` | sidebar rail: AI | hugeicons `SparklesIcon` |
| `crates/ui/src/app_shell.rs:854` | ☰ `\u{2630}` | sidebar toggle button | hugeicons menu / `SidebarLeftIcon` |
| `crates/ui/src/app_shell.rs:913` | 🛡️ `\u{1F6E1}` | AI-access shield badge | hugeicons `ShieldIcon` |
| `crates/ui/src/ai_chat.rs:428` | ✂ `\u{2702}` | attachment: selection | hugeicons `TextIcon` |
| `crates/ui/src/ai_chat.rs:429` | 📄 `\u{1F4C4}` | attachment: file | hugeicons `File02Icon` |
| `crates/ui/src/ai_chat.rs:430` | 🖼️ `\u{1F5BC}` | attachment: image | hugeicons `Image01Icon` |
| `crates/ui/src/ai_chat.rs:936` | 📎 `\u{1F4CE}` | attachment chip | hugeicons `Attachment01Icon` |
| `crates/ui/src/sftp.rs:1289` | 📁 `\u{1F4C1}` | sftp dir row | Catppuccin `folder` |
| `crates/ui/src/sftp.rs:1291` | 🔗 `\u{1F517}` | sftp symlink row | hugeicons `LinkIcon` |
| `crates/ui/src/sftp.rs:1293` | 📄 `\u{1F4C4}` | sftp file row | Catppuccin `file` |
| `crates/ui/src/snippets.rs:1483` | ✎ `\u{270E}` | edit snippet | hugeicons `PencilEdit02Icon` |
| `crates/ui/src/snippets.rs:1496` | ⧉ `\u{29C9}` | duplicate snippet | hugeicons `Copy01Icon` |
| `crates/ui/src/snippets.rs:1508` | 🗑️ `\u{1F5D1}` | delete snippet | hugeicons `Delete02Icon` |
| `crates/ui/src/snippets.rs:2214` | 🔍 `\u{1F50D}` | snippet search empty | hugeicons `Search01Icon` |
| `crates/ui/src/git.rs:2370` | ✎ `\u{270E}` | rename/edit affordance | hugeicons `PencilEdit02Icon` |
| `crates/ui/src/git.rs:2860`, `sftp.rs:1082`, `git_graph.rs:847` | ↻ `\u{21BB}` | refresh buttons | hugeicons `Refresh01Icon` |

**Geometric/technical glyphs used as icons — should become real icons:**

| file:line | glyph | used for |
|---|---|---|
| `crates/ui/src/tabs.rs:48-56` | ⌂ ▸ ✎ ◈ ✦ ⇅ ⎇ ± | per-`TabKind` tab-bar indicator (reference uses hugeicons SVGs) |
| `crates/ui/src/hosts.rs:46` | ⚠ `\u{26A0}` | host status: Failed |
| `crates/ui/src/hosts.rs:1851,1853,2031,2033` | ☑ ☐ `\u{2611}/\u{2610}` | checkbox states (reference uses a real checkbox control) |
| `crates/ui/src/notifications.rs:54` | ⚠ `\u{26A0}` | Warning severity |
| `crates/ui/src/snippets.rs:2016` | ⚠ `\u{26A0}` | run status: Error |
| `crates/ui/src/workspace.rs:2418, 106, 3197` | ⚠ `\u{26A0}`, `⤳ \u{2933}` | host-key-changed banner, jump-host arrow |

Non-icon decorative uses that are fine to keep: `→ ↔ ✓ · — …` in log/status strings, box-drawing `│`, `▾/▸` disclosure triangles (arguably should be chevron icons for parity but low priority).

### 2c. Concrete fixes (icons)

1. **Add a real icon system.** Either add `gpui-component` (it ships an `Icon`/`IconName` set backed by Lucide SVGs) as the architecture doc intends, or vendor the specific Hugeicons SVGs the reference uses into `crates/ui/assets/icons/` and render with `gpui::svg().path(...)`. Given the parity rule, matching Hugeicons names is closest to the reference.
2. **Port the Catppuccin file-icon resolver.** Bundle `@iconify-json/catppuccin` icon bodies as SVG assets + port `iconResolver.ts` / `fileIcons.ts` / `folderIcons.ts` / `EXT_TO_LANGUAGE_ID` to Rust lookup tables; `explorer.rs::file_glyph` and `sftp.rs` row icons become `svg()` calls. This is the single biggest visual-parity gap in the explorer/SFTP.
3. Replace every row in the table above. Remove `TabKind::indicator()` glyphs in favour of per-kind SVGs matching `tabUtils.tsx`.
4. Replace `hosts.rs` checkbox glyphs with a real checkbox element; replace ⚠ severity glyphs with a warning SVG tinted `status_warning`.
5. Update the `explorer.rs` unit tests (`:1819-1821`) that assert on emoji code points.

---

## 3. Theme system

### 3a. Reference

- **Theme file model** (`src-tauri/src/modules/themes/mod.rs`): `Theme { name, author, author_url, version, description, variants: HashMap<String, ThemeVariant> }`; `ThemeVariant { mode: "light"|"dark", label?, colors: HashMap<token,string> }`. A theme may have **many** variants of the same mode (Catppuccin: `latte`=light, `frappe`/`macchiato`/`mocha`=dark).
- **Backend commands**: `theme_get_default`, `themes_get_all` (scans `config_dir()/themes/*.json`), `theme_import`, `theme_export`, `theme_delete` (built-ins protected), **`theme_create`** (new file seeded from default, returns path to open in editor), `themes_get_dir`, **`theme_fetch_index`** (community index JSON via reqwest), **`theme_download`** (fetch + validate + save from a raw URL).
- **`useThemeStore.ts`**: `installedThemes`, `communityThemes` (+ `MOCK_COMMUNITY_THEMES` fallback: Catppuccin, Nord; index URL `raw.githubusercontent.com/Snenjih/labonair-themes/main/index.json`), `installingIds`, `previewThemeId`. Actions: `fetchInstalled`, `fetchCommunity`, `installTheme`, `uninstallTheme`, `applyTheme(id, variantKey?)`, `previewTheme(meta|null, variantKey?)`, `cancelPreview`, `createTheme(name)`.
- **Variant override persistence**: `themeVariantOverrides[themeId][resolvedMode] = variantKey` (`setThemeVariantOverride`), consulted on apply and on palette preview (`useSettingsCommands.ts:220-233`, `useThemeStore.ts:applyTheme`).
- **Live theme switching**: `applyThemeColors`/`revertThemeColors` (`lib/useThemeEngine.ts`) mutate CSS custom properties at runtime; the palette themes page previews on hover (`onPreview`) and reverts on leave (`onLeave`).
- **Color mode**: `theme` pref = `dark|light|system` → `resolvedMode`; drives which variant is picked.
- **Editor syntax themes**: `EDITOR_THEMES` / `EDITOR_THEME_LABELS` (auto-follows app theme by default).

### 3b. Port

`crates/theme/**` + `crates/ui/src/theme.rs` + `crates/ui/src/settings.rs`:
- `ThemeFile` / `ThemeFileVariant` (`crates/theme/src/import.rs:41-66`) mirror the reference struct **including** multi-variant maps and `label` — **but** `resolve_variant(dark)` (`:100-107`) just returns **the first variant whose `mode` matches**, or any. There is **no `variantKey` selection and no `themeVariantOverrides` persistence** — Catppuccin would always render as `frappe` (or whichever sorts first in the `BTreeMap`), with no UI to choose `mocha`.
- `ThemeStore` (`crates/ui/src/theme.rs:159-355`): built-in `light`/`dark`, `ThemePreference` (System/Light/Dark), **exactly one** `custom` theme (`custom` / `custom_base` / `custom_file`), `import_theme_file`, `clear_custom_theme`, `active_theme_file` (export), `FontOverrides`, `EditorThemeId` (7 syntax themes). No concept of an installed-theme list inside the store.
- `settings.rs` **does** implement: `refresh_themes` / `scan_themes(themes_dir())`, `activate_theme(id)` (reads file, `import_theme_file`), `import_theme` (file picker → copy into themes dir), `import_theme_from`, export, `ThemeEntry` list with built-in-protected flag, `active_theme_id`. So a **local** installed-theme list exists in the Settings pane.
- **Missing entirely**: `theme_fetch_index` / `theme_download` / community index / `MOCK_COMMUNITY_THEMES` / install / uninstall-from-marketplace; `theme_create` ("New Theme from default"); `themes_get_dir` "open folder"; per-theme **variant picker** + `themeVariantOverrides`; **palette** theme commands (`Change App Theme…`, `Change Color Mode…`, `Change Editor Theme…` and their sub-pages) — none are in `COMMANDS`; **live hover-preview** of a theme (`onPreview`/`onLeave`).

### 3c. Concrete fixes (theme)

1. **Variant selection.** Add `active_variant: BTreeMap<String /*themeId*/, BTreeMap<String /*mode*/, String /*variantKey*/>>` persisted in preferences; change `ThemeFile::resolve_variant` to take an optional `variant_key`; expose a variant picker in the Appearance pane (and the palette themes sub-page) listing every variant with its `label`.
2. **Community/marketplace.** Port `theme_fetch_index` + `theme_download` into `crates/backend` (reqwest is already a dep), add `MOCK_COMMUNITY_THEMES` fallback and the `Snenjih/labonair-themes` index URL, and a "Community Themes" section in the Appearance pane with install/uninstall + `installing` state.
3. **`theme_create`.** "New Theme…" button → seed a file from the default, save to themes dir, open it (native editor tab).
4. **Palette theme commands.** Add `Change App Theme…` (→ `themes` sub-page: built-in default + installed, `on_preview` live-apply, `on_leave` revert), `Change Color Mode…` (→ Dark/Light/System), `Change Editor Theme…` (→ `EDITOR_THEMES`), each with `rightLabel: "active"`.
5. **Live preview** helper on `ThemeStore` (`preview_theme(Option<&ThemeFile>, variant)` / `cancel_preview`) that swaps the resolved theme without touching persisted `active_theme_id`.
6. Multi-installed model: keep `ThemeStore.custom` as "the resolved active theme" but let it be sourced from any installed file id, not just an imported one (settings.rs already does most of this — surface it through the palette).

---

## 4. Dynamic sidebars

### 4a. Reference

`statusbar/lib/useSidebar.ts` + `sidebarSlotLogic.ts` + `app/components/SidebarContent.tsx` / `WorkspaceArea.tsx`:
- **Two independent dock slots** — `primary` (legacy keys `sidebarOpen`/`sidebarActivePanel`/`sidebarWidth`, follows the `sidebarPosition` pref for which screen edge) and `secondary` (`sidebarRightOpen`/`sidebarRightActivePanel`/`sidebarRightWidth`, always the opposite edge, closed by default). `left`/`right` are derived from `sidebarPosition` (`useSidebar.ts:` "const left = sidebarPosition === 'right' ? secondary : primary").
- **Any panel dockable to any side.** `SidebarPanel = "explorer" | "snippets" | "source-control" | "git-graph" | "ai" | "tabs" | "hosts" | null`. Per-panel placement in `barItemPlacements[itemId] = { side, hidden, ... }` via `PANEL_TO_ITEM_ID`; `resolveSide(panel, side?)` falls back: explicit arg → registered per-item side → global `sidebarPosition`.
- **API**: `handlePanelToggle(panel, side?)`, `openPanel(panel, side?)` (show, don't toggle-closed — used by palette/menu), **`movePanel(panel, from, to)`** (collapse in old slot, `move` into new, displacing whatever was there), `onLayoutChanged` (debounced width persist, 300ms).
- **State machine per slot** (`useSidebarSlot`): restore-from-prefs once `prefsHydrated` (collapse/expand/resize to stored width), persist `open` + `panel` on every `activePanel` change, `onResize` keeps `activePanel` in sync with manual drags (`resolveResize`: collapse < 1% clears panel, drag-open restores last-active), `expand()` resizes to a self-tracked `lastOpenWidthPx` (not the panel lib's fragile pre-collapse memory), debounced `persistWidth`.
- `resolveToggle` (`sidebarSlotLogic.ts:35-52`): click active panel → collapse (or re-expand if dragged to 0); click other panel → switch (+ expand if collapsed). Pure + unit-tested (`sidebarSlotLogic.test.ts`).
- Special case: when `tabsLocation` leaves `"sidebar"`, any slot showing `"tabs"` falls back to `"explorer"`.

### 4b. Port

`crates/ui/src/app_shell.rs`:
- **One** sidebar, **left edge only** (`//! an optional, resizable left sidebar`, `:8-9`). No right/secondary slot anywhere (`grep right` in `app_shell.rs` finds only geometry, no dock).
- `SidebarPanel` enum has **5** variants: `Explorer`, `Snippets`, `SourceControl`, `GitGraph`, `Ai` (`:73-89`). **Missing** `Tabs` (tabs-in-sidebar) and `Hosts`.
- State: `sidebar_open: bool`, `sidebar_width: f32`, `active_panel: SidebarPanel` — plain fields, **no persistence** (init `sidebar_open: true`, `SIDEBAR_DEFAULT`, `Explorer` at `:407-409`; not restored from or written to preferences).
- `toggle_sidebar` (`:446`), `select_panel` (`:451-458` — click active + open → close, else switch+open; a partial `resolveToggle`), `set_sidebar_width` (`:461-465`, clamps, no persist).
- Rail render (`:1059-1110`): fixed vertical strip of the 5 glyphs, left border, no drag between sides, no per-item side, no "move to other side".
- No `movePanel`, no `openPanel` vs `toggle` distinction (palette `CommandId::ToggleAiPanel` etc. call `select_panel`, which *will* toggle-closed if already active — wrong for "show me X" intent), no debounced width persistence, no restore effect, no `barItemPlacements`, no `sidebarPosition`, no drag-to-collapse healing.

### 4c. Concrete fixes (sidebars)

1. **Add a second dock slot.** Refactor `AppShell` sidebar state into a reusable `SidebarSlot { open, width, active_panel, last_open_width }` and hold two (`primary`, `secondary`); derive `left`/`right` from a new `sidebar_position` preference.
2. **Persist** `open` / `active_panel` / `width` per slot in preferences (keys mirroring `sidebarOpen`/`sidebarActivePanel`/`sidebarWidth` + `sidebarRight*`), restore on startup, debounce width writes (~300ms).
3. **Port `sidebarSlotLogic`** (`resolve_toggle`, `resolve_resize`, `is_collapsed`) as a pure unit-tested module; use it in `select_panel` and the resize handler.
4. **Add `move_panel(panel, from, to)`** and a per-panel placement map (`bar_item_placements`); add a rail affordance (context menu "Move to Right/Left", matching `BarItemContextMenu.tsx`) to move a panel between slots.
5. **Split `open_panel` (show) from `toggle_panel`** — palette/menu commands (`ToggleAiPanel`, `OpenSnippetsPanel`, `OpenGitGraph`, `FocusSourceControl`) that mean "show X" must not close it when it's already active. (Reference: `openPanel` vs `handlePanelToggle`.)
6. **Add the `Tabs` and `Hosts` sidebar panels** (tabs-in-sidebar is gated on a `tabsLocation` preference; add the "leaving sidebar → fall back to explorer" rule).

---

## 5. Context menus — complete gap table

Legend: ✅ ported (may differ in styling), ⚠️ partial, ❌ missing.

| # | Menu (reference file) | Reference items | Port status |
|---|---|---|---|
| 1 | **Workspace tab** `tabs/components/WorkspaceTabContextMenuContent.tsx` | Rename, Duplicate, (Close Others / Close All ⟨kind⟩ / Close — when >1 tab) | ⚠️ `workspace.rs:2948` — Close, Close Others, Close All Of This Type, Grant AI Agent Access. **Missing Rename, Duplicate.** Label wording differs ("Close All Of This Type" vs "Close All ⟨kind⟩"). |
| 2 | **Non-workspace tab** `tabs/components/NonWorkspaceTabContextMenuContent.tsx` | Keep Tab Open (peek tabs), Duplicate Tab, Close Others, Close All, Close All ⟨kind⟩, Close Tab | ⚠️ same single `workspace.rs` menu for all kinds. **Missing Keep Tab Open, Duplicate Tab, Close All.** |
| 3 | **Tab bar empty area** `tabs/TabBar.tsx`, `tabs/SidebarTabList.tsx` | (new-tab / layout actions via `tabUtils` dropdown) | ❌ no empty-area tab-bar menu in port. |
| 4 | **Terminal pane** `terminal/TerminalPane.tsx:137-159` | Copy (disabled w/o selection), Paste, Clear, —, Ask AI about Selection | ❌ **No menu.** `terminal.rs:467` right-click just pastes. |
| 5 | **SSH terminal pane** `terminal/SshTerminalPane.tsx:700-736` | Copy, Paste, Clear, —, Ask AI about Selection | ❌ missing. |
| 6 | **File tree node** `explorer/FileTreeNode.tsx:210-286` | (file) Open, Open Preview, Reveal in Terminal, Bookmark Path, Reveal in Finder, Change Permissions…; New File, New Folder, Copy Path, Copy Relative Path, Attach to AI Agent, Rename, Delete | ⚠️ `explorer.rs:1424` — New File, New Folder, Rename, Delete, Copy, Cut, Paste, Copy Path, Open in Terminal, Open in Preview, Bookmark This Folder. **Missing: Open, Reveal in Finder, Copy Relative Path, Attach to AI Agent, Change Permissions/chmod-chown.** Extra (not in ref): Copy/Cut/Paste clipboard ops. |
| 7 | **Explorer root** `explorer/FileExplorer.tsx:570-600` | Reveal in Terminal, Reveal in Finder, New File, New Folder, Copy Path, Refresh, Hard Refresh | ⚠️ folded into the node menu; **Reveal in Finder, Hard Refresh missing at root.** |
| 8 | **SFTP file list** `sftp/components/SftpContextMenu.tsx:170-262` | New Folder, New File, Rename, Download to…, Upload files here…, Bookmark, Refresh, Open (double-click target), Copy Path(s), Properties…, Edit (open remote), Delete | ⚠️ `sftp.rs:1431` — New Folder, New File, Rename, Copy Path, Delete…, Download to Local / Upload to Remote, Permissions…, Properties…, Edit Remote File, Refresh. **Missing: Bookmark, Open.** Wording differs ("Download to Local" vs "Download to…"). |
| 9 | **Source-control file change** `source-control/components/FileChangeItem.tsx:218-253` | Unstage / Stage, Discard Changes (destructive), Add to .gitignore, Add to .git/info/exclude, Open Diff, Open Diff (Split) | ❌ **No context menu** on `git.rs` file rows (only a ✎ affordance at `:2370`). |
| 10 | **Git graph commit** `git-graph/components/GitGraphCanvas.tsx:262-290` | View Changes, —, Checkout (detached HEAD), Create Branch Here…, —, Cherry-pick, —, Copy Hash, Copy Short Hash | ❌ missing in `git_graph.rs`. |
| 11 | **Host card** `hosts/components/HostCard.tsx:405-443` (+ bulk) | Connect SSH, Open SFTP, Edit, Duplicate, Pin/Unpin, Export to SSH Config, Delete (destructive); bulk: Connect SSH (n), Open SFTP (n), Duplicate (n), Delete (n) | ❌ missing in `hosts.rs`. |
| 12 | **Host list item** `hosts/components/HostListItem.tsx:309-341` | same as HostCard (list layout) | ❌ missing. |
| 13 | **Host group card** `hosts/components/GroupCard.tsx:105-117` | Rename Group (inline), Delete Group (destructive) | ❌ missing. |
| 14 | **Snippet item** `snippets/components/SnippetItem.tsx:205-239` | Run in Terminal, Run Silent, Run (Inject), Copy Command, Edit, Duplicate, Delete (destructive) | ❌ `snippets.rs` has inline ✎/⧉/🗑 buttons only, no right-click menu with the run-mode variants. |
| 15 | **CWD breadcrumb** `statusbar/CwdBreadcrumb.tsx:200-252` | ⟨segment label⟩, Copy Path, Copy Relative Path, cd here, cd in New Tab, Open in Finder (conditional), Reveal…/extra | ❌ no statusbar breadcrumb context menu in port. |
| 16 | **Status-bar item (bar item)** `settings/components/BarItemContextMenu.tsx` | Side ▸ (Left / Right radio), Location ▸ (Titlebar / Statusbar radio), Hide | ❌ missing — ties into the dynamic-sidebar gap (§4). |
| 17 | **Appearance section** `settings/sections/AppearanceSection.tsx` (`onContextMenu`) | (bar-item preview right-click → BarItemContextMenu) | ❌ missing. |
| 18 | **SFTP pane / VirtualizedFileList** `sftp/SftpPane.tsx`, `sftp/components/VirtualizedFileList.tsx` | delegates to SftpContextMenu (empty-space vs. row) | ⚠️ `sftp.rs:1140` handles empty-space right-click but see #8 gaps. |

**Styling note:** the reference `components/ui/context-menu.tsx` (radix) gives every menu: `min-w-[8rem]`, `rounded-md`, `border`, `bg-popover`, `p-1`, `shadow-md`, items `rounded-sm px-2 py-1.5 text-sm`, `focus:bg-accent`, destructive variant = `text-destructive`, separators, sub-menus, radio/checkbox items, and open/close zoom+fade animation. The port's three ad-hoc menus (`explorer.rs`, `sftp.rs`, `workspace.rs`) each re-implement a `div` with `px_2 py_1 text_sm hover:bg(border)` — no shared component, no separators, no destructive styling, no sub-menus, no animation, positioned inconsistently (`explorer.rs` hard-codes `top: 26px; left: 10px` instead of anchoring at the cursor; `workspace.rs` correctly uses `pos.x/pos.y`).

### 5c. Concrete fixes (context menus)

1. **Build one shared `ContextMenu` component** in `crates/ui` (anchored at cursor, backdrop-dismiss, `min-w 128px`, `rounded-md`, `bg(card)`, `border`, `p-1`, shadow, items `rounded-sm px-2 py-1.5 text-sm` + `hover:bg(accent)`, `Separator`, destructive = `text(status_error)`, disabled state, optional leading icon, sub-menus, radio/checkbox items). Reuse it for the 3 existing menus.
2. Add the **11 missing menus** (#4, #5, #9, #10, #11, #12, #13, #14, #15, #16, #17) and fill the **gaps** in #1, #2, #6, #7, #8.
3. Fix `explorer.rs` menu anchoring to the cursor position (like `workspace.rs`).
4. Add the destructive treatment for Delete / Discard / Delete Group / Delete Host.

---

## 6. Prioritised fix list

**P0 — the visible "it looks broken" items (user complaints):**
1. Rebuild the command-palette view to reference metrics (§1c.1): 640px width, 40px rows, section headings, left accent bar, footer, empty-state copy. Fixes "cramped un-styled list".
2. Add the icon system (§2c.1) + port the Catppuccin file-icon resolver (§2c.2) + replace all emoji/glyph icons (§2c.3–4). Fixes "emojis everywhere".
3. Make the sidebar dynamic: second dock slot + persistence + `move_panel` + `open` vs `toggle` (§4c.1–5). Fixes "sidebars not dynamic".
4. Command-palette section grouping: the palette currently groups by the static `section` string with substring search only — implement the 11 sub-pages + breadcrumb (§1c.3) so "Switch Tab / Themes / Branches / Snippets…" stop being flat/mis-grouped.

**P1 — functional parity gaps:**
5. Dynamic command registry — port the 14 hook modules, add Quick Connect / per-host / per-snippet / per-session / per-branch / outline / ~17 settings toggles / git actions (§1c.4).
6. Palette theme commands + theme variant selection + `themeVariantOverrides` (§3c.1, §3c.4).
7. Theme community/marketplace + `theme_create` (§3c.2–3).
8. Shared `ContextMenu` component + the 11 missing menus (§5c).
9. Palette: icons, subtitle, `rightLabel` (ON/OFF/active), recents, fuzzy/startsWith search, footer search-mode toggle, live theme preview (§1c.5–7, §1c.10).

**P2 — polish / correctness:**
10. Palette preferences (blur/opacity/position/animation/close-on-overlay/show-recent/search-mode) wired into Settings (§1c.8).
11. `ShortcutsOpen` binding `cmd-k` vs reference `cmd-?` — reconcile (§1b).
12. Destructive-item styling, menu anchoring fix, separators (§5c.3–4).
13. Replace `hosts.rs` checkbox glyphs and ⚠ severity glyphs with real controls/icons (§2c.4).
14. Update emoji-asserting unit tests in `explorer.rs`.
