//! Sub-section grouping per top-level category (`SECTION_GROUPS`), mirroring the
//! group headers in `reference-src/src/settings/sections/*`. Split out of the
//! old `crates/ui/src/settings.rs` monolith in T16-007 (mechanical move — no
//! logic change).

/// Sub-section layout per top-level category, mirroring the group headers in
/// `reference-src/src/settings/sections/*`. `render_grouped` walks these in
/// order; any field not named here still renders under a trailing "Other".
type FieldGroup = (&'static str, &'static [&'static str]);
pub const SECTION_GROUPS: &[(&str, &[FieldGroup])] = &[
    (
        "General",
        &[
            (
                "Startup",
                &["defaultStartupTab", "startupTerminalCount", "autostart"],
            ),
            (
                "Session Restore",
                &[
                    "sessionRestore",
                    "sessionScrollbackLines",
                    "scrollbackMaxSizeMb",
                    "scrollbackRetentionDays",
                ],
            ),
            ("Window", &["restoreWindowState"]),
            ("Security", &["credentialEncryption"]),
            ("Quit", &["confirmQuitWithSsh"]),
            ("Updates", &["checkForUpdates"]),
            ("Notifications", &["notifyOnErrors"]),
        ],
    ),
    (
        "Terminal",
        &[
            (
                "Shell",
                &[
                    "terminalShell",
                    "terminalDefaultPath",
                    "newTabInheritsCwd",
                    "confirmCloseTerminalTab",
                ],
            ),
            (
                "Font",
                &[
                    "terminalFontFamily",
                    "terminalFontSize",
                    "terminalFontWeight",
                    "terminalLineHeight",
                    "terminalLetterSpacing",
                ],
            ),
            (
                "Cursor",
                &[
                    "terminalCursorStyle",
                    "terminalCursorBlink",
                    "terminalCursorBlinkInterval",
                ],
            ),
            (
                "Layout",
                &["terminalShowPaneHeader", "terminalShowPaneFooter"],
            ),
            (
                "Composer & Blocks",
                &[
                    "terminalComposerEnabled",
                    "terminalComposerHistoryPopup",
                    "terminalComposerArgumentCompletion",
                    "terminalBlocksEnabled",
                    "terminalBlocksAutoCollapseOnAltScreen",
                ],
            ),
            ("Bell", &["terminalBell"]),
            ("Buffer", &["terminalScrollback"]),
            (
                "Input",
                &[
                    "terminalCopyOnSelect",
                    "terminalRightClickPastes",
                    "terminalWordSeparator",
                ],
            ),
            (
                "Scrolling",
                &["terminalScrollSensitivity", "terminalFastScrollModifier"],
            ),
            ("Appearance", &["terminalOpacity"]),
        ],
    ),
    (
        "Editor",
        &[
            ("Keybindings", &["vimMode", "editorRelativeLineNumbers"]),
            ("Theme", &["editorTheme"]),
            (
                "Font",
                &["editorFontFamily", "editorFontSize", "editorLineHeight"],
            ),
            (
                "Behaviour",
                &[
                    "editorFormatOnSave",
                    "editorAutoSave",
                    "editorAutoSaveDelay",
                    "editorTabSize",
                ],
            ),
            ("Indentation", &["editorIndentWithTabs"]),
            ("Files", &["editorMaxFileSizeMb"]),
            (
                "Display",
                &[
                    "editorLineNumbers",
                    "editorWordWrap",
                    "editorBracketMatching",
                    "editorShowCursorPosition",
                    "editorShowSelectionStats",
                    "editorShowOutline",
                    "editorIndentationGuides",
                ],
            ),
            (
                "On Save",
                &["editorTrimTrailingWhitespace", "editorInsertFinalNewline"],
            ),
            ("AI Completion", &["editorAutocompleteDebounceMs"]),
        ],
    ),
    (
        "File Manager",
        &[
            (
                "Browsing",
                &[
                    "sftpShowHiddenFiles",
                    "sftpShowUpFolder",
                    "explorerShowHiddenByDefault",
                ],
            ),
            (
                "Columns",
                &[
                    "sftpColumnSize",
                    "sftpColumnModified",
                    "sftpColumnPermissions",
                    "sftpColumnType",
                ],
            ),
            (
                "Remote Editing",
                &[
                    "sftpRemoteEditShowTransfers",
                    "sftpMaxRemoteFileSizeMb",
                    "sftpFontSize",
                ],
            ),
            (
                "Transfers",
                &[
                    "sftpMaxConcurrentTransfers",
                    "sftpDefaultConflictResolution",
                    "sftpChunkSizeKb",
                    "sftpOnFolderFileError",
                ],
            ),
        ],
    ),
    (
        "Connections",
        &[
            ("Host Availability", &["hostPingInterval"]),
            (
                "SSH Terminal Sessions",
                &[
                    "sshConnectTimeoutSecs",
                    "sshAutoReconnect",
                    "sshAutoReconnectDelay",
                    "sshAutoReconnectMaxAttempts",
                ],
            ),
            (
                "Remote File Browsing",
                &[
                    "explorerRemotePollInterval",
                    "explorerAutoReconnect",
                    "explorerIdleSessionTimeoutMin",
                    "explorerMaxIdleSessions",
                    "explorerMaxCachedRemoteScopes",
                ],
            ),
        ],
    ),
    (
        "Workspace",
        &[
            (
                "Bookmarks",
                &[
                    "bookmarksEnabled",
                    "bookmarksActionNewTerminal",
                    "bookmarksActionCurrentTerminal",
                    "bookmarksActionCurrentSftp",
                    "bookmarksActionNewSftp",
                    "bookmarksPrimaryClickBehavior",
                    "bookmarksShowBadge",
                ],
            ),
            (
                "Command Palette",
                &[
                    "commandPaletteBlur",
                    "commandPaletteOpacity",
                    "commandPalettePosition",
                    "commandPaletteAnimation",
                    "commandPaletteShowRecent",
                    "commandPaletteHistorySize",
                    "commandPaletteSearchMode",
                    "commandPaletteCloseOnOverlayClick",
                ],
            ),
            ("Source Control", &["gitStatusPollIntervalMs"]),
        ],
    ),
    (
        "AI",
        &[
            (
                "Defaults",
                &[
                    "defaultModelId",
                    "autocompleteEnabled",
                    "autocompleteProvider",
                    "autocompleteModelId",
                ],
            ),
            ("General", &["aiEnabled", "aiWarnDestructiveCommands"]),
            (
                "Behaviour",
                &[
                    "aiAutoOpenMiniOnSend",
                    "aiNotifyOnHeadlessCommand",
                    "aiMaxAgentSteps",
                    "aiTemperature",
                    "aiTerminalContextLines",
                    "aiShellMaxTimeoutSecs",
                    "aiShellMaxOutputKb",
                ],
            ),
            ("Agents & Directives", &["customInstructions"]),
        ],
    ),
];
