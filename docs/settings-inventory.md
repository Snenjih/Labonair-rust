# Settings Field Inventory

**Status:** Normative for R05-001  
**Owner:** `labonair-settings-content` (values), `labonair-settings` (layers and persistence), `labonair-settings-ui` (value editor)  
**Last reviewed:** 2026-09-07

This inventory is the gate for changing `SettingsContent`. A field may remain
in the model only when its owning runtime consumer, default, validation, and
scope are known. A field may be exposed in the Settings UI only when it is in
the **Keep** column. Persisted theme IDs are values; theme registries and theme
selection remain owned by `labonair-theme`.

## Scope vocabulary

| Scope | Meaning |
|---|---|
| Global | Shared by the application and stored in the user settings file. |
| Project | Safe to commit in `.labonair/settings.json`; it must be on the project whitelist. |
| Runtime state | Current layout/session state; it belongs to the workspace/state owner, not Settings. |
| Capability-owned | Belongs with a feature registry/store and must not become a Settings category. |
| Migration-only | Read only from legacy files; never a current UI field. |

## Decision vocabulary

- **Keep** — has a current consumer and remains a generated Settings field.
- **Move** — has a valid consumer, but the current location duplicates or
  obscures the owner; replace it with an explicit migration and contract.
- **Remove** — no current consumer or no supported product workflow. Preserve
  old input through the migration/unknown-field policy before deleting code.
- **Review** — evidence is incomplete; do not treat the field as supported.

## Inventory

Field names are the exact JSON keys exposed by the typed model. “Consumer”
means a runtime module, not merely serialization or the generated Settings UI.

| Area | Fields | Consumer / scope | Decision |
|---|---|---|---|
| `general` | `theme` | `labonair-theme` color-mode preference; Global | Keep |
| `general` | `restoreWindowState`, `defaultStartupTab`, `sessionRestore` | shell/workspace launch and session restore; Global + Project where whitelisted | Keep |
| `general` | `startupTerminalCount`, `autostart`, `credentialEncryption`, `confirmQuitWithSsh` | no current native runtime consumer or supported workflow | Remove; legacy input remains deserializable only |
| `general` | `checkForUpdates` | updater launch policy; Global | Keep |
| `appearance` | `appTheme`, `iconTheme`, `themeVariantOverrides` | theme registry selection persistence; Global; capability-owned registry | Keep as values, no management page |
| `appearance` | `appFontFamily`, `appFontSize`, `appLineHeight`, `bufferFontFamily`, `bufferFontSize`, `bufferLineHeight`, `uiDensity`, `cornerRadiusScale`, `reduceMotion` | theme metrics/font pipeline; Global | Keep |
| `appearance` | `backgroundImage`, `backgroundOpacity`, `backgroundBlur`, `backgroundTintColor`, `backgroundTintOpacity` | `labonair-background` store; Global | Removed from Settings model; values migrate to the Background owner |
| `appearance` | `appCornerRadius` | superseded by `cornerRadiusScale`; no current consumer | Remove; legacy values migrate to `cornerRadiusScale` |
| `appearance` | `tabsLocation`, `zenModeShowHeader`, `zenModeShowStatusbar` | shell/workspace presentation; Global | Keep pending direct consumer tests |
| `appearance` | `sidebarTabInfoLine`, `sidebarGroupByFolder`, `sidebarGroupSingleTabs`, `badgesAlwaysVisible`, `titlebarsIconsPosition` | no current native consumer; titlebar position is legacy | Remove; legacy input remains deserializable only |
| `terminal` | `terminalShell`, `terminalDefaultPath`, `newTabInheritsCwd`, `confirmCloseTerminalTab` | terminal/workspace tab creation and close flow; Global + safe Project values | Keep |
| `terminal` | `terminalFontFamily`, `terminalFontSize`, `terminalLineHeight`, `terminalScrollback`, `terminalCursorStyle`, `terminalCursorBlink`, `terminalCopyOnSelect`, `terminalRightClickPastes`, `terminalBell`, `terminalOpacity` | terminal renderer/settings adapter; Global + safe Project values | Keep |
| `terminal` | `sessionScrollbackLines`, `scrollbackMaxSizeMb`, `scrollbackRetentionDays` | scrollback persistence/cleanup; Global | Keep |
| `terminal` | `terminalFontWeight`, `terminalLetterSpacing`, `terminalCursorBlinkInterval`, `terminalWordSeparator`, `terminalScrollSensitivity`, `terminalFastScrollModifier` | no current Settings consumer; theme typography values are theme-owned | Remove; legacy input remains deserializable only |
| `terminal` | `terminalShowPaneHeader`, `terminalShowPaneFooter` | terminal view presentation; Global | Keep |
| `terminal` | `terminalUseWebgl` | impossible in native GPUI terminal path; legacy WebView option | Remove; legacy input remains deserializable only |
| `terminal` | `terminalComposerEnabled`, `terminalComposerHistoryPopup`, `terminalComposerArgumentCompletion`, `terminalBlocksEnabled`, `terminalBlocksAutoCollapseOnAltScreen` | no supported native workflow or consumer | Remove; legacy input remains deserializable only |
| `editor` | `editorFontFamily`, `editorFontSize`, `editorLineHeight`, `editorTabSize`, `editorWordWrap`, `editorLineNumbers`, `editorRelativeLineNumbers`, `editorIndentWithTabs` | native editor settings adapter; Global + Project where whitelisted | Keep |
| `editor` | `editorFormatOnSave` | editor command/action surface; Global + Project where safe | Keep |
| `editor` | `editorLineHeight`, `editorTrimTrailingWhitespace`, `editorInsertFinalNewline`, `editorBracketMatching`, `editorShowCursorPosition`, `editorShowSelectionStats`, `editorShowOutline`, `editorIndentationGuides` | no current native editor consumer | Remove; legacy input remains deserializable only |
| `editor` | `editorAutoSave`, `editorAutoSaveDelay`, `editorAutocompleteDebounceMs`, `editorMaxFileSizeMb` | no current native editor consumer | Remove; legacy input remains deserializable only |
| `editor` | `editorVimMode`, `vimHlsearch`, `vimIncsearch`, `vimSmartcase` | editor Vim runtime; Global + safe Project values | Keep |
| `editor` | `editorTheme` | syntax-theme selection persistence; Global + safe Project values | Keep as value, no management page |
| `fileManager` | `explorerShowHiddenByDefault`, `explorerIndentGuides`, `explorerStickyAncestors`, `explorerAutoRevealActiveFile`, `explorerFoldSingleChildDirs`, `explorerGitDecorations`, `scmFileTree` | Explorer/SCM owners; Global + safe Project values | Keep |
| `fileManager` | `sftpShowHiddenFiles`, `sftpShowUpFolder`, `sftpColumnSize`, `sftpColumnModified`, `sftpColumnPermissions`, `sftpColumnType`, `sftpFontSize` | no current native SFTP-browser Settings consumer | Remove; legacy input remains deserializable only |
| `fileManager` | `sftpRemoteEditShowTransfers`, `sftpMaxRemoteFileSizeMb`, `sftpMaxConcurrentTransfers`, `sftpDefaultConflictResolution`, `sftpChunkSizeKb`, `sftpOnFolderFileError` | runtime transfer policy belongs to the SFTP/transfer owner | Remove from Settings; future owner defines its own typed policy |
| `workspace` | `commandPaletteSearchMode`, `commandPaletteShowRecent`, `commandPaletteHistorySize`, `commandPaletteOpacity`, `commandPalettePosition`, `commandPaletteCloseOnOverlayClick` | command-palette runtime; Global | Keep as palette values, no registration state |
| `workspace` | `commandPaletteBlur`, `commandPaletteAnimation` | no current native palette consumer | Remove; legacy input remains deserializable only |
| `workspace` | `gitStatusPollIntervalMs` | no current native SCM polling consumer | Remove; legacy input remains deserializable only |
| `workspace` | `dockLayout`, `sidebarPosition`, `sidebarOpen`, `sidebarActivePanel`, `sidebarRightOpen`, `sidebarRightActivePanel`, `sidebarWidth`, `sidebarRightWidth` | workspace layout/session state; Runtime state | Removed from `SettingsContent`; `labonair-workspace` imports these legacy keys into `workspace-layout.json` |

## Required follow-up order

1. Add focused consumer tests for every **Keep**/**Review** field whose runtime
   owner is currently indirect.
2. Keep Background and workspace layout state behind their owning module
   contracts, preserving existing user data with explicit migrations. Both
   owner paths are active; remaining work is consumer proof and removal of
   other legacy fields.
3. Remove confirmed legacy/unsupported fields from the typed model and UI;
   retain unknown legacy JSON according to the migration policy.
4. Rebuild the project whitelist from this inventory instead of maintaining a
   second unrelated list.

Until those steps are complete, new Settings fields require an inventory row
before code changes.

## Unmodelled legacy input

The previous shipped default contained `general.notifyOnErrors`, but no typed
field or runtime consumer existed for it. It was removed from the shipped
default in this slice. The new `SettingsContent` test rejects any future
untyped key in the default asset, so stale configuration cannot silently look
supported again.
